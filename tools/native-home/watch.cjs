'use strict';
const fs = require('fs');
const cp = require('child_process');
const readline = require('readline');
const APP = 'io.github.justmaaarco.aerialhome.nativeprobe';
const native = require('./open-native.cjs');
const EXPECTED_RELEASE = 'Rockhopper release 9.2.4-48 (ombre-okapi)';
const POWER_URI = 'luna://com.webos.service.tvpower/power/getPowerState';
const LAUNCH_URI = 'luna://com.webos.applicationManager/launch';
const OFF = new Set(['Power Off', 'Suspend', 'Active Standby', 'Standby']);

// Screen-off and screensaver events do not constitute a new power-on.
class PowerState {
  constructor(launchOnInitial) {
    this.state = null;
    this.armed = launchOnInitial;
    this.initial = true;
  }
  update(msg) {
    if (!msg || msg.returnValue === false || typeof msg.state !== 'string') return false;
    this.state = msg.state;
    const poweringOff = /power\s*off|suspend|standby/i.test(String(msg.processing || ''));
    if (OFF.has(msg.state) || poweringOff) {
      this.armed = true;
      this.initial = false;
      return false;
    }
    if (msg.state !== 'Active') return false;
    const trigger = this.armed;
    this.armed = false;
    this.initial = false;
    return trigger;
  }
}
module.exports = { PowerState };

function main() {
  if (process.getuid() !== 0) throw Error('Root required');
  if (fs.readFileSync('/etc/starfish-release', 'utf8').trim() !== EXPECTED_RELEASE) {
    throw Error('Firmware changed; power-on integration needs review');
  }
  const metadata = JSON.parse(fs.readFileSync('/media/developer/apps/usr/palm/applications/' + APP + '/appinfo.json'));
  if (metadata.id !== APP || metadata.type !== 'native' || metadata.version !== '0.2.0') throw Error('Expected native 0.2.0 installation missing');
  native.validateNative();
  const logFile = '/tmp/luma-power-events.log';
  function log(event, data) {
    try {
      if (fs.existsSync(logFile) && fs.statSync(logFile).size > 65536) fs.renameSync(logFile, logFile + '.previous');
      fs.appendFileSync(logFile, JSON.stringify({at: new Date().toISOString(), event, ...data}) + '\n', {mode: 0o600});
    } catch (_) {}
  }
  const machine = new PowerState(!process.argv.includes('--no-initial-launch'));
  let monitor, monitorTimer, launchTimer, launching = false, generation = 0, stopped = false;
  const children = new Set();
  function request(uri, payload, timeout) {
    return new Promise(resolve => {
      const child = cp.spawn('/usr/bin/luna-send', ['-n', '1', '-w', String(timeout), uri, JSON.stringify(payload)], {stdio: ['pipe', 'pipe', 'pipe']});
      children.add(child);
      let data = '', errors = '', completed = false;
      const timer = setTimeout(() => child.kill('SIGTERM'), timeout + 1500);
      child.stdout.on('data', x => { data += x; if (data.length > 262144) child.kill('SIGTERM'); });
      child.stderr.on('data', x => { errors = (errors + x).slice(-1024); });
      function done(error) {
        if (completed) return;
        completed = true; clearTimeout(timer); children.delete(child);
        child.stdin.destroy();
        try { resolve(JSON.parse(data)); } catch (_) { resolve({returnValue: false, errorText: error || errors || 'No JSON reply'}); }
      }
      child.once('error', e => done(e.message));
      child.once('close', () => done());
      // Keep stdin open: this firmware's interactive luna-send exits at stdin EOF.
    });
  }
  function schedule(reason) {
    const g = ++generation;
    clearTimeout(launchTimer);
    log('power-on-detected', {reason});
    launchTimer = setTimeout(async () => {
      if (stopped || g !== generation || machine.state !== 'Active' || launching) return;
      launching = true;
      try {
        for (let attempt = 1; attempt <= 3; attempt++) {
          if (stopped || g !== generation || machine.state !== 'Active') break;
          let result;
          try {
            result = await native.openNative({
              children, request,
              canLaunch: () => !stopped && g === generation && machine.state === 'Active'
            });
          } catch (error) {
            result = {returnValue:false, errorText:error.message};
          }
          log('launch-result', {attempt, success: result.returnValue === true, error: result.errorText});
          if (result.returnValue === true) break;
          await new Promise(r => setTimeout(r, 2000));
        }
      } finally { launching = false; }
    }, 0);
  }
  function subscribe() {
    if (stopped) return;
    const child = cp.spawn('/usr/bin/luna-send', ['-i', POWER_URI, '{"subscribe":true}'], {stdio: ['pipe', 'pipe', 'pipe']});
    monitor = child; children.add(child);
    const lines = readline.createInterface({input: child.stdout});
    let received = false;
    const firstReply = setTimeout(() => { if (!received) child.kill('SIGTERM'); }, 15000);
    lines.on('line', line => {
      let msg; try { msg = JSON.parse(line); } catch (_) { return; }
      if (!msg.state || msg.returnValue === false) { log('subscription-rejected', {error: msg.errorText}); child.kill('SIGTERM'); return; }
      received = true; clearTimeout(firstReply);
      const old = machine.state;
      const fire = machine.update(msg);
      if (old !== msg.state || msg.processing) log('power-state', {state: msg.state, processing: msg.processing});
      if (msg.state !== 'Active' || /power\s*off|suspend|standby/i.test(String(msg.processing || ''))) {
        generation++; clearTimeout(launchTimer);
      }
      if (fire) schedule(old === null ? 'full-startup' : 'standby-wake');
    });
    child.stderr.on('data', x => log('subscription-error', {error: String(x).slice(-512)}));
    child.once('error', e => log('subscription-error', {error: e.message}));
    child.once('close', () => {
      clearTimeout(firstReply); lines.close(); children.delete(child); child.stdin.destroy();
      log('subscription-ended', {});
      if (!stopped) monitorTimer = setTimeout(subscribe, 5000);
    });
  }
  function stop() {
    stopped = true; generation++; clearTimeout(launchTimer); clearTimeout(monitorTimer);
    for (const child of children) child.kill('SIGTERM');
    log('stopped', {}); setTimeout(() => process.exit(0), 250);
  }
  fs.writeFileSync('/tmp/luma-power.pid', String(process.pid), {mode: 0o600});
  process.once('SIGTERM', stop); process.once('SIGINT', stop);
  log('started', {pid: process.pid, launchOnInitial: machine.armed});
  subscribe();
}
if (require.main === module) main();
