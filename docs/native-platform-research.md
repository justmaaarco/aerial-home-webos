# Native platform requests and lifecycle

This is a source review and local implementation note. It does not establish that the TV's observed app-switch delay has been fixed. A deployed package and another controlled hardware benchmark are still required.

## What the benchmark established

In two slow native trials, SAM began launching YouTube about 120–164 ms after the injected Enter request started. YouTube's window became visible 2.39–2.97 seconds **after SAM began launching it**. The snapshots showed no active native media pipeline during those immediate-app flows. These observations locate the longer delay after initial launch dispatch; they do not identify which SAM, surface-manager or native lifecycle stage was waiting.

The request worker has separate artwork and media-helper workers. The UI queues video visibility and app-launch commands without waiting for video unload. A hidden rendering loop that keeps polling is another possible source of competition and needs its own validation; it should not be assumed to explain these particular timings.

## LS2 identity and app lifecycle are separate concerns

The [LS2 library reference](https://www.webosose.org/docs/reference/luna-service2-library/luna-service2-library-api-reference/#lsregister) permits `LSRegister(NULL)` for a client that does not receive incoming service methods. Its anonymous name is not, by itself, evidence of incorrect permissions or app lifecycle registration.

`LSRegisterApplicationService` is not a safe universal replacement for native clients. The [upstream LS2 hub](https://github.com/webosose/luna-service2/blob/5bc38aa348b35f5799f1bf9ed7c6cb513bc5f702/src/ls-hubd/hub.cpp#L1082) checks that a peer supplying an application ID is an application container; other executable clients are rejected. Executable-based permissions are resolved separately in the [active permission map](https://github.com/webosose/luna-service2/blob/5bc38aa348b35f5799f1bf9ed7c6cb513bc5f702/src/ls-hubd/active_permission_map.cpp#L166). This implementation keeps its existing executable identity and does not change roles, permissions or application IDs.

OSE's [native app overview](https://www.webosose.org/docs/guides/development/native-apps/native-app-overview/) describes SAM `registerApp` as the native lifecycle connection. That is separate from registering an LS2 client handle. OSE also [warns that its APIs are not guaranteed to match LG retail products](https://www.webosose.org/docs/reference/).

The current [SDL-webOS implementation](https://github.com/webosbrew/SDL-webOS/blob/ba4c86af391d1524d3ff0cb61dc3b10035933239/src/core/webos/SDL_webos_init.c#L60) owns lifecycle registration: it selects `registerNativeApp` or `registerApp` from the app's lifecycle interface version, subscribes, and raises its window on relaunch. Its current hint is `SDL_WEBOS_REGISTER_APP`. Retail SDL versions may differ. The exact installed TV library must be checked before relying on a particular hint or registering an additional subscription; an SDK library is not evidence of the retail runtime's behavior.

## Bounded waits without frequent polling

The native LS2 request worker now blocks on its own GLib context instead of polling it every 2 ms. An owned 6.5-second timeout source wakes that same context if no reply arrives. The request thread still owns the handle, context and callback storage, and timeout cleanup happens before callback storage is released.

This follows GLib's documented [blocking context iteration](https://docs.gtk.org/glib/method.MainContext.iteration.html), [monotonic timeout source](https://docs.gtk.org/glib/func.timeout_source_new.html), and [source destruction/reference rules](https://docs.gtk.org/glib/method.Source.destroy.html). Linux host tests exercise a real GLib context, verify that the timeout wakes a blocking wait, and verify removal of a pending source before its callback storage is freed. They do not exercise the TV's LS2 server or native lifecycle.

## Optional launch diagnostics

`--trace-platform` or `AERIAL_NATIVE_TRACE=1` enables JSON lines on standard error. `native-launch-timing` records the target app ID, process ID, wall-clock timestamp, elapsed duration and these phases:

- `worker` / `started` and `finished`: the request worker starts and finishes the launch operation.
- `ls2` / `submitted`, `reply` or `timeout`: direct LS2 call submission and completion.

The trace does not log service payloads, media URLs, preferences or artwork paths. A successful reply acknowledges the request; it does not prove that the destination window or app content is ready. Correlate these phases with SAM and surface-manager events during a later authorized TV test. Leave diagnostics off for the primary performance comparison unless both solutions use equivalent instrumentation.
