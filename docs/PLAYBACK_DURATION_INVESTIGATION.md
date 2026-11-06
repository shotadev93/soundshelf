# WebKit playback duration and pause investigation

Date: 2026-11-04. Scope: the v0.1.0 candidate's optional native regression failure, `invalid duration 0`. Existing release-preparation changes were preserved. Only synthetic audio and temporary indexes were used; no personal library was opened or modified.

**Subsequent manual finding:** the user reproduced a separate application control regression in the installed original Debian package: play → pause → seek → resume → pause fails. It is now reproduced and fixed in the actual controller as recorded in [the follow-up below](#confirmed-application-pause-regression). The duration-only investigation's successful raw-media tests did not establish that application controls worked after seeking.

## Finding

The reported duration failure is a test-harness false failure triggered by transient WebKit/GStreamer metadata timing. It is not evidence that the fixture has zero duration. No application decoding or transport regression was identified for the tested files. This finding does not certify the packaged application's GUI, audible output, all codecs or all Linux systems.

The unchanged test first passed all six format/origin combinations, confirming intermittency. An instrumented repetition then reproduced the failure on MP3 from the production document origin during its fourth matrix pass. At that point WebKit exposed:

```text
durationchange: duration=0, readyState=4
loadedmetadata: duration=0, readyState=4
loadeddata: duration=10.152, readyState=4
canplay: duration=10.152, readyState=4
playing: duration=10.152, readyState=4
```

After 1.5 seconds of diagnostic observation the same media element still reported 10.152 seconds, without a decoding error. In a later successful run of the corrected test, development-origin MP3 also reported zero through `loadeddata` and recovered at `canplay`. A second `durationchange` was not required for recovery in either trace. The platform-internal reason for this event ordering has not been established; it should not be described as a proven upstream source-code defect.

The original probe checked `Math.abs(duration - 10) > 0.3` once inside `loadedmetadata` and failed immediately on zero. Conversely, that expression would not reject NaN. It also waited for metadata before calling play, unlike the application's controller, which starts playback and observes timing events. The application already updates duration on `loadedmetadata`, `durationchange` and `timeupdate`, so its timing state recovers during playback. A new controller regression covers zero metadata followed by a positive duration and timeupdate without a second durationchange.

The [HTML media specification](https://html.spec.whatwg.org/multipage/media.html) defines duration changes and media readiness events; the probe now observes subsequent media events rather than assuming the first duration read is final. This is a compatibility treatment of the observed runtime behavior, not a claim that zero at loadedmetadata is required or universally expected.

## Fixtures and independent metadata

`gst-discoverer-1.0` independently analyzed disposable equivalents of the native test's 10-second, mono, 8 kHz fixtures:

| Format | Duration | Seekable |
| --- | --- | --- |
| 16-bit PCM WAV | 10.000000000 s | yes |
| GStreamer lamemp3enc MP3 | 10.152000000 s | yes |
| Vorbis/OGG | 10.000000000 s | yes |

The MP3 encoder padding remains within the original 0.3-second tolerance; that tolerance was not widened. The WAV RIFF/data sizes represent 160,000 PCM bytes (160,044 bytes including the header), not an empty file. MP3 and OGG are encoded from that WAV using the existing encoders. No fixture replacement was needed to obtain a pass.

The existing GUI fixture generator was also run unchanged. Its mono 44.1 kHz tones independently measured approximately 19.969161 s (WAV/OGG) and 20.044321 s (MP3), all seekable. These are generated-fixture metadata checks, not audible GUI playback results.

## Transport and development/production comparison

The internal `soundshelf-audio://localhost/<token>` locator is converted to `http://127.0.0.1:<ephemeral-port>/audio/<token>` before reaching the media element. The application does not register the old custom audio scheme for WebKit media loading. No file URL or broad asset scope was introduced.

The native regression now compares actual full HTTP bodies and byte ranges against the original bytes for WAV, MP3 and OGG, rather than checking only the WAV response length and zero-filled slices. Full, boundary/suffix ranges, 416, HEAD, invalid tokens and foreign origins remain covered. All three original byte arrays are compared after playback. Correct transport bodies and independently valid durations rule out a truncated/zero-length fixture or corruption by the tested streaming adapter as the cause of this failure.

Debug tests exercise both `http://localhost:1420` and `tauri://localhost` document origins. These are real WebKit documents loaded by the harness, not a full React/Tauri GUI session or an actual Vite server. Release-profile tests exercise only the production document origin, because release authorization intentionally rejects the development Origin. The route test now asserts that distinction in both profiles; it has not been relaxed to admit development origins in production.

## Test changes

- `src-tauri/tests/support/webkit-playback.mjs`: a test-only duration gate shared by the WebKit probe and deterministic Node tests. It requires a finite, positive duration within 0.3 seconds of the expected value, has a five-second failure deadline, handles media errors, and removes listeners/timers when settled. It listens to metadata, duration, loadeddata, canplay and timing events.
- `src/lib/webkit-playback.test.mjs`: tests delayed duration without a second durationchange, already known/later durations, persistent zero, NaN, infinity, incorrect finite duration and media errors.
- `src/lib/playback-controller.test.mjs`: verifies the existing application controller recovers from transient zero metadata through playback timing, and keeps seeking disabled while duration is unknown.
- `src-tauri/src/playback_protocol.rs`: changes are confined to `cfg(test)` modules. The native probe starts like the real controller, waits for valid duration and actual time advancement, checks pause stability, seeks to five seconds, resumes and verifies further advancement. It emits bounded event diagnostics on success/failure and retains per-case deadlines. All three codecs must be present in the test index.

During probe development, awaiting the resumed `play()` promise before observing time advancement caused an additional diagnostic timeout: WebKit advanced currentTime after seeking while leaving the promise pending until later readiness. The final probe, like the original probe, observes advancement independently and still reports play rejections. It does not claim to verify prompt play-promise settlement or the full application's control state after every seek. Record paused/loading/control behavior during the manual acceptance below.

In the initial duration-only investigation, no production playback, filesystem, IPC, database, CSP, capability or dependency behavior changed. The inline script allowance belongs only to the pre-existing isolated test document; production CSP was not changed. Test helpers are included only in the native test build and Node tests. No audio file has been added to the repository.

## Verification record

The initial duration failure was reproduced with diagnostics; one sandbox attempt separately failed GTK initialization. Desktop runs used the existing approved native-test command. Missing WebVTT encoder/fakevideosink warnings persisted during successful audio runs; installing plugins was not required to correct the duration assertion, and no system package was installed.

Final results are recorded in the [release checklist](RELEASE_CHECKLIST.md#playback-duration-follow-up). The previously built Debian artifact and its checksum remain the original release-preparation artifacts; no package installation or publication is implied by this test investigation.

## Remaining manual acceptance

Use the isolated XDG directories and disposable fixture procedure in the [release checklist](RELEASE_CHECKLIST.md#manual-packaged-linux-acceptance-reference). Test both the actual development application and the installed Debian candidate with Vite stopped for the latter. Record OS, WebKitGTK/GStreamer versions, exact package checksum and outcomes.

1. Select and scan the generated directory. Confirm WAV, MP3 and OGG duration becomes positive and plausible; a briefly unknown value must recover without a stuck loading/error state.
2. At low volume confirm audible output, pause/resume, forward/backward/end seeking, replay and switching formats. After seeking, confirm Play/Pause and seek controls remain usable and the displayed loading/paused state matches actual playback. Native muted advancement does not prove audible output or rendered control behavior.
3. Try corrupt/empty and externally missing copies and recover to valid audio. Verify errors are recoverable and existing metadata is retained.
4. Restart with the same isolated data directory; confirm persistence and native-picker reauthorization. Compare the untouched fixture hashes/names/modes/mtimes and check no sidecars were created.
5. Complete the checklist's package uninstall/reinstall checks without deleting existing data. AppImage acceptance remains unavailable until its separate missing-patchelf packaging issue is resolved on a provisioned build host.

No commit, tag, push, upload or release publication was performed.

## Confirmed application pause regression

The user confirmed the installed Debian reproduction. Before changing production code, two deterministic controller regressions failed, and the real WebKit test was upgraded to run the application's controller rather than directly calling `audio.pause()`. That test also failed with `controller pause after seek/resume failed`. Its MP3 development-origin trace showed currentTime advancing to 5.267225 seconds after seeking, while the controller remained `loading`. This confirms an application state/control bug distinct from the earlier premature-duration assertion.

The cause had three connected parts:

1. `toggle()` set `phase = loading` while awaiting a resumed `play()` promise. WebKit can advance audio after seeking before this promise or a new `playing` event arrives.
2. `AudioPlayer` enabled its Play/Pause button only for `playing`/`paused`, and `toggle()` paused only in the `playing` phase. Consequently both the button and controller refused a pause during that pending/loading period. A `waiting` event could produce the same condition after an already successful play call.
3. The existing generation guard distinguished selected tracks but not repeated play/pause attempts on one track. A later rejection of an interrupted older play could overwrite a newer pause/resume with an error. Queued `pause`/`playing` events were also accepted without checking the media element's current paused state.

The fix adds explicit `playRequested` intent to the internal frontend snapshot, independent of readiness/loading. Pause remains enabled while a play request is pending or media is buffering. Pause synchronously cancels that intent, invalidates the play attempt, calls `audio.pause()` and publishes paused state. Play results require both the selected-track generation and the current play-attempt number. Delayed events are reconciled against current media properties and intent; actual media errors still stop playback and remain visible. Known-duration seeking remains usable during a pending play request. No IPC field or Rust authorization behavior changes.

The native probe now builds `PlaybackController` and its actual imports in memory through the existing Vite dependency (`scripts/build-playback-test-module.mjs`). It uses real WebKit audio and the existing authorized streaming route, drives the exact reported sequence, verifies stable time after the second pause, and repeats rapid seek/play/pause/resume operations. Node plus installed frontend dependencies are therefore prerequisites for this optional test; no Vite server or additional package is needed. It still does not mount React or establish audible output.

Four new deterministic controller tests cover the exact pending-promise sequence, repeated actions with older promises resolving/rejecting out of order, queued events, buffering, Stop and real decoder errors. The original cancellation, autoplay, codec-error, stale-selection and duration tests remain. The upgraded native test failed before the production fix and passed all six debug format/origin cases afterward. Final verification and the separate corrected Debian candidate are recorded in the [release checklist](RELEASE_CHECKLIST.md#pause-after-seekresume-regression).

The first test-bundling experiment rejected Vite's single-entry output array; the test helper was corrected before reproducing the actual controller failure. A first targeted Node invocation omitted the repository's `--test-isolation=none` flag and was rerun with the established option. These setup failures were not application regressions.

Filesystem containment, native-picker authorization, playback tokens, the loopback adapter, CSP, capabilities, database location/schema and dependency versions remain unchanged. Existing release artifacts are retained. The old pre-fix Debian package still contains the defective frontend and must not be published as the fixed candidate.

## User-reported corrected Debian acceptance

In the final pre-release audit request, the user reported installing and testing the corrected Debian candidate, playback regressions passing **6/6** (including pause → seek → resume → pause, repeated seeks, rapid toggles and track switching), audible WAV/MP3/OGG output, launch without Vite, passing scanning/search/tags/favorites/persistence/reauthorization, and preserved application data after reinstallation. These are user-reported manual results, not independently executed by Codex. They supersede the earlier pending status for those cases; they do not add a measured artifact hash, runtime log, manual media-integrity snapshot or independent screen-reader test. See the release checklist for current legal/asset packaging decisions and the final candidate status.
