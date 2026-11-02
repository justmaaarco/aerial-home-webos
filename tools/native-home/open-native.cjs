'use strict';
// Aerial Home Native Home/power helper. This resets only our native view and
// asks SAM to foreground it; it never selects HDMI or changes power settings.
const fs = require('fs');
const cp = require('child_process');

const APP = 'io.github.justmaaarco.aerialhome.nativeprobe';
const DIRECTORY = '/media/developer/apps/usr/palm/applications/' + APP;
const BINARY = DIRECTORY + '/aerial-native-probe';
const LAUNCH_URI = 'luna://com.webos.applicationManager/launch';

function validateNative() {
  const file = DIRECTORY + '/appinfo.json';
  if (fs.statSync(file).size > 65536) throw Error('Native manifest is too large');
  const info = JSON.parse(fs.readFileSync(file, 'utf8'));
  if (info.id !== APP || info.type !== 'native' || info.version !== '0.2.0' || info.main !== 'aerial-native-probe') {
    throw Error('Expected separate Aerial Home Native 0.2.0 installation');
  }
  if (!fs.statSync(BINARY).isFile()) throw Error('Native executable is missing');
  fs.accessSync(BINARY, fs.constants.R_OK | fs.constants.X_OK);
  return info;
}

function run(executable, args, timeout, children, jsonReply) {
  return new Promise(resolve => {
    let child, completed = false, output = '', errors = '';
    function done(result) {
      if (completed) return;
      completed = true;
      clearTimeout(timer);
      if (child) {
        if (children) children.delete(child);
        child.stdin.destroy();
        child.stdout.destroy();
        child.stderr.destroy();
        if (child.exitCode === null) child.kill('SIGTERM');
        child.unref();
      }
      resolve(result);
    }
    const timer = setTimeout(() => done({returnValue:false, errorText:'Native Home request timed out'}), timeout);
    try {
      child = cp.spawn(executable, args, {stdio:['pipe','pipe','pipe']});
      if (children) children.add(child);
      child.stdout.on('data', bytes => {
        output += bytes;
        if (output.length > 262144) return done({returnValue:false, errorText:'Native Home reply exceeded its limit'});
        if (jsonReply) {
          try { done(JSON.parse(output)); } catch (_) {}
        }
      });
      child.stderr.on('data', bytes => { errors = (errors + bytes).slice(-2048); });
      child.once('error', error => done({returnValue:false, errorText:error.message}));
      child.once('close', code => {
        if (!jsonReply && code === 0) return done({returnValue:true});
        if (jsonReply) {
          try { return done(JSON.parse(output)); } catch (_) {}
        }
        done({returnValue:false, errorText:errors || 'Native Home child exited with code ' + code});
      });
      // Keep stdin open until completion. This TV's luna-send may otherwise
      // exit at stdin EOF before submitting its request.
    } catch (error) { done({returnValue:false, errorText:error.message}); }
  });
}

function signalHome(children) {
  return run(BINARY, ['--home'], 2500, children, false);
}
function request(uri, payload, timeout, children) {
  return run('/usr/bin/luna-send', ['-n','1','-w',String(timeout),uri,JSON.stringify(payload)], timeout + 1500, children, true);
}
async function openNative(options) {
  options = options || {};
  (options.validate || validateNative)();
  const canLaunch = options.canLaunch || (() => true);
  if (!canLaunch()) return {returnValue:false, errorText:'Power transition cancelled'};
  const reset = await (options.signal || signalHome)(options.children);
  if (reset.returnValue !== true) return reset;
  if (!canLaunch()) return {returnValue:false, errorText:'Power transition cancelled'};
  const launch = options.request || ((uri, payload, timeout) => request(uri, payload, timeout, options.children));
  return launch(LAUNCH_URI, {id:APP}, 5000);
}

module.exports = {APP, BINARY, validateNative, signalHome, openNative};
if (require.main === module) {
  if (process.argv.includes('--check')) {
    try { console.log(JSON.stringify({checked:true, app:validateNative().id, actions:['binary --home','SAM launch']})); }
    catch (error) { console.error(error.message); process.exitCode = 1; }
  } else {
    openNative().then(result => {
      console.log(JSON.stringify(result));
      if (result.returnValue !== true) process.exitCode = 1;
    }, error => { console.error(error.message); process.exitCode = 1; });
  }
}
