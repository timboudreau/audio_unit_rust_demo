Rust Audio Unit Demo
====================

This is a *minimal* demo of doing DSP processing in Rust for an Apple Audio Unit.  It builds
a tiny Rust library that implements a Gain/Pan plugin, and uses it from two Audio Unit
implementations.  And it serves as a demo of how to (after configuring various certificates
and ids and whatnot), generate usable signed Mac OS installers in a generic way that, with
some effort, can be used for other projects.

The installers generated by the `audiounits-package` script are for independent distribution,
not the Apple App Store.  It ought to be relatively simple to change the flavor of certificate used
to target the App Store instead if desired.

The installer-generation script requires configuring several app-ids, certificates and provisioning
profiles [in your Apple account](https://developer.apple.com/account/resources/profiles/list) and
using Xcode to set the projects to use them - all of the steps are described below.

Overview
========

This project was created to

 * Tease out issues with automating builds of AudioUnit plugins
 * Sort out the rearranging of sources and projects required to make an AudioUnit able to run
   in-process (Apple's template is not set up to do this, and the documentation makes a cursory
   reference to moving the code into a Framework and having the application extension point to
   it, but nowhere does there exist an open source example of how you actually *do* this, and it
   is non-obvious and non-trivial - there are two plugin projects here - `RustDspPlainDemo` just
   uses Apple's template as-is, while `RustDspInProcessDemo` is that project, rearranged to move
   all of the code into a Framework project)
   * Side note - after several days of work debugging this and finally getting it to work, I
     discovered that Logic Pro will not load AUv3 audio-units in-process anyway.  But perhaps someday
     it will
  * Sort out the vagaries of getting an installer to properly get an AudioUnit registered - all of
    the documentation says to place them in `/Library/Audio/Plug-Ins/Components`, which does not
    appear to work, while simply bunding the semi-useless demo app which is part of the AUv3 template
    causes it to magically be registered - I'd guess due to some filesystem watching black magic,
    but there is no documentation of how it happens
  * Attempt tog et a fully automated, **command-line** build of multiple audio units working soup-to-nuts, from
    code to signed, notarized, valid, usable installers or dmgs

I have, at present, 13 AudioUnit projects with the DSP done in Rust (most of the audio-unit C++, Swift
and Objective C code I generate from attributes in the Rust code) - releasing all of these by pressing
buttons in an IDE and hoping I don't forget a step, get things out-of-order or mix up the myriad
certificates, app-ids and other fussy bits is a non-starter.  Apple doesn't make it *easy* to do this
stuff outside of Xcode, but it's not entirely impossible given a little one-time set-up in Xcode.

What's Where
------------

### Rust DSP Code

Is in `/crates/mock_dsp_lib` - it is just a dirt-simple plugin that does stereo-only gain and pan, and
is *decidedly* not written with safety or sanity checking in-mind, but it's only here to have a thing
to build a plugin around.

The C-ABI bindings are generated by the `crates/cbindgen-bindgen` binary project, which is just a wrapper
around [cbindgen](https://github.com/mozilla/cbindgen) that knows what to do and where to put things.

The rust-build heavy-lifting is done by the script in the checkout root `build-xcframeworks` which

 * Builds the rust code
 * Builds `cbindgen-bindgen` if it is not present in `$CHECKOUT_ROOT/bin` (it is very slow to build
   and unlikely to change) and puts it there
 * Generates C bindings into the build `target/` dir
 * Builds the rust DSP library once for `aarch64` and once for `x86_64`
 * Merges the results into a single universal binary with `lipo`
 * Creates an XCFramework for the resulting static library and places it in `macos/xcframeworks` where
   the xcode projects can import it from)
 * If the environment variable `CODE_SIGN_ID` is set, signs the XCFramework with it - to find the right
   value for you, run `security find-identity -v -p codesigning` and set the variable to the hash printed
   on the left sign of the line for the appropriate certificate
   * This step is likely unnecessary - the framework is a statically linked library, and there is no need
   to bundle it, but it is here for completeness' sake, and because you may have XCFrameworks you *want*
   to ship.

### XCode Audio Unit Projects

So, there are two Audio Unit plug-in projects in the `macos/` directory.  The first, `RustDspPlainDemo`,
is simply the simplest possible implementation of an Audio Unit that calls out to Rust for DSP.  The second,
`RustDspInProcessDemo` makes the following changes based on [these instructions](https://developer.apple.com/documentation/avfaudio/audio_engine/creating_custom_audio_effects#4302110):
  * A Framework target is added
  * All sources and copied resources are moved from the application extension target's Compile Sources phase to that target (they are in the same locations relative to the project root, just compiled by a different project target)
  * The various C and C++ headers are included in a copy task (as they are in Apple's AudioUnit template - however
    nonituitive it is, this seems to be the way you use such headers in a project without the Objective C build
    tripping over them, trying to parse them as Objective C headers and failing - why the C++ compiler finds
    them *at all* is a mystery to me, but xcodebuild is a foot-deep layer of lipstick on a 1990s pig, so why not?)
  * A single dummy Swift source is added to the Framework target, per the instructions, so there is *something* to compile,
    because reasons.
  * The `Info.plist` of the app extension has the entries described in the link above added to it


Prerequisites
-------------

 * Mac OS 13 or newer + an SDK for Mac OS 13 (or perhaps later if you tweak `build-xcframeworks`)
 * Xcode 14 or newer + command-line tools (`xcodebuild` and `lipo` are used)
 * A Rust toolchain including `cargo` - built with Rust 1.80.1 - whatever `rustup` gives you that supports 2021
   edition or newer should be fine


Quickstart
----------

 1. Build the rust libraries by running `build-xcframeworks` (if you don't, the audio unit plugins will be
    missing a library).
   * This builds the Rust library for Intel and Apple Silicon, combines them into a universal binary and
     builds an XCFramework around them which can be loaded by the plugins
 2. Open the Xcode projects in `macos/` and build and run:
   * `RustDspPlainDemo` is the as-unmodified-as-possible Apple Audio Unit Template modified to call
     the Rust library built by `build-xcframeworks` to do the DSP heavy-lifting
   * `RustDspInProcessDemo` is the a revision of the above to move all of the code into a Framework and leave
     the Audio Unit app extension as an empty shell that delegates to the Framework, so that it can
     be (but isn't currently - the problem I'm trying to solve) loaded in-process by the host

Note that you will need to set these projects to be signed with *your* credentials to actually build anything
(IMHO signing is completely orthagonal to developing and building software, but I guess I see the convenience
of shoving it into IDE projects, given that the command-line tooling to do this same is excruciating to work
with).


Details
-------

To deviate as little as possible from the Apple Audio Unit template, the following are the modifications
made to the default code:

 * `RustDspPlainDemoExtensionAudioUnit` configures itself only to support stereo, overrides `canProcessInPlace`,
    and contains a spinlock/busywait to avoid a race condition on deinit in the original Apple code
 * `ObservableAUParameter` exposes the parameter address so the UI can switch on it conveniently, to support
    a small tweak in `ParameterSlider` to convert values to decibels for gain
 * `RustDspPlainDemoExtensionDSPKernel` calls the Rust code, and uses some Rust code to do its parameter reads
    and writes atomically


Signing And Notarizing
----------------------

While it's hard to imagine anyone wanting to endure the steps below, this project is designed as a working
template for how to do this stuff.  So the best approach to begin using it with your own code is to get
a checkout of this project building usable installers, and once you have something you know works, then
start applying it to your own code.

It is not necessary or important that what you're doing involves Rust for DSP - simply both that and
signing, notarizing etc. audio plugins are both ferociously fussy to get working and have non-obvious
failure modes.

### Pitfalls

A few issues encountered in the week+ it took to get this working:

 * While the aim of this project is to create fully automatable command-line builds, some aspects of
   signing simply *must* be configured once in Xcode.
 * If there is any chance you have more than one *Developer ID Application* certificate installed on
   your machine, you **must** explicitly specify *the same specific one* to all of the targets for
   signing.  The "Provisioning Profile" combo-box on the Signing & Capabilities page of Xcode's project settings
   will not let you do this.  Signing > Code Signing Identity in Xcode's Build Settings will let you
   specify exactly the certificate to use.
   * Failure to do this will result in inscrutable error messages at runtime such as
   *external subsystem [NSViewService_PKSubsystem] not present*, or *Unable to find NSExtensionPrincipalClass*
   which have nothing to do with the actual problem.
 * The *provisioning profiles* you set up on Apple's web site and use for signing **must** use exactly
   the same certificate - a mismatch here will also result in failures that give no indication of the
   actual problem
   
### Prerequisites

Some resources need to be created in your Apple account for this to work.  I do not know if it is
necessary to change the bundle-ids of the projects avoid collisions - I would assume not, but I do
not have a second developer account to test with.

You need to set up some SSL certificates:

 * An SSL certificate of the *Developer ID Application* flavor, which you create [here](https://developer.apple.com/account/resources/certificates/list)
 and then need to download, install in your keychain, and possibly tell Xcode about it in Xcode >
 Settings > Accounts > Manage Cetrificates.
 * An SSL certificate of the *Developer ID Installer* flavor, created in the same way
 
and two "app identifiers" which can be created [here](https://developer.apple.com/account/resources/identifiers/list).
The app-id contains the bundle-id of a binary.  You need

 * One for the app ID `com.mastfrog.RustDspInProcessDemo` - for the installer-generation script to
 work, this *must* be given the simple-name `RustDspInProcessDemo`, as the installer-generation script
 assumes a convention that for each bundle-ID it encounters, there is a corresponding app-id named with
 the tailing `.`-delimited element
 
 * One for the app ID `com.mastfrog.RustDspInProcessDemo.RustDspInProcessDemoExtension` which *must*
 be named `RustDspInProcessDemoExtension` for the installer to work.
 
The *framework* target gets signed by `RustDspInProcessDemoExtension` so it does not need its own
app-id, but it **must** be set to use *exactly* the same team and signing certificate as the other
targets, or cryptic errors will haunt you at runtime.

Two *Provisioning Profiles* which you create [here](https://developer.apple.com/account/resources/profiles/list).
As best I can tell, a provisioning profile simply ties together an app-id and a certificate.  These are:

 * A profile named `RustDspInProcessDemo` which ties the app-ID named `RustDspInProcessDemo` to the
 *Developer ID Application* certificate you will sign with (make sure it's the same one as used in the
 build!)
 * A profile named `RustDspInProcessDemoExtension` which ties the app-ID named `RustDspInProcessDemoExtension`
 to the same certificate
 
Download the profiles, and on the Signing & Capabilities page for the app and application-extension projects,
click Provisioning Profile (after disabling *Automatically manage signining* if it is checked) and choose
*Import Profile* to pull it in, once for each project (the same profile cannot be used for both).


### Naming Conventions

If you are reusing these build scripts, the installer and signing phases rely on several hard-and-fast
conventions that allow ids and names and identifiers to be derived from the project name and its bundle
ID.  If you deviate from these, the packaging script must be modified to know what to do.

 * A project consists of three targets that will be nested in each other - if the main project is
   named, say, SidewaysSound
   1. The project folder is named SidewaysSound
   2. The name of the app-extension target is SidewaysSoundExtension
   3. The framework containing the DSP code is named SidewaysSoundFramework
     * Any other miscellaneous frameworks (such as the Rust XCFramework we build here) will be statically
       linked into the final binary, and they do not need to be signed or included
   4. The bundle ID of the app ends with "SidewaysSound"
   5. The bundle ID of the extension ends with "SidewaysSound.SidewaysSoundExtension", and the leading elements
      (e.g. `com.foo.dsp`) are the same for both the app and the extension
   6. The bundle ID of the ends with "SidewaysSoundFramework"
   7. There are App-Ids registered with Apple for **both** the app and the extension, using the above
      described bundle IDs.  Each one's simple name (which you set in the UI on Apple's web site) is
      the simple name of the target / trailing .-delimited element of the bundle-id, e.g. SidewaysSound
      and SidewaysSoundExtension
   8. There are app-specific password credentials already stored in your keychain named "notarytool"
   
### Environment Variables

Certificate hashes, team IDs and similar, used in the build to specify credentials to use when signing
are set.  The required ones are:

 * `APP_SIGNING_ID` - the hash of a developer-id certificate such as "EE3E05AAE04537BA0813CFCB76F35AE832432EBB"
 * `DISTRIBUTION_SIGNING_ID` - this will be a hash of a certificate, such as "EE3E05AAE04537BA0813CFCB76F35AE832432EBB",
 findable using the `security` cli app, and *will not be the same as the `APP_SIGNING_ID` certificate* - Apple
 requires separate and separately registered certificates for applications versus installers.
 * `CODE_SIGN_ID` - The hash of a certificate, such as "3E2A53EF135C0D57EEA876F980A8986A7B0B372E"
 * `NOTARIZATION_TEAM_ID` - a team-id such as "QR2HR8VQP7"
 
 

### Signing Process

In `RustDspInProcessDemo`, signing the three (app, app extension, library) targets is configured as follows - if
you reconfigure the projects to use your certificates, these are the things to change

 * The `RustDspInProcessDemo` (app) and `RustDspInProcessDemoExtension` (.appex - no code) targets are both
   configured to use *the same* **Developer ID Certificate** (we are targeting generating installers for
   non-app store distribution - if we were targeting the app store it would need a different flavor of
   certificate)
 * Both of those targets need separate App IDs registered
 * The `RustDspInProcessDemoFramework` target is set *not to be signed*
 * The `RustDspInProcessDemoExtension` is configured with **Other Signing Flags** set to **`--deep`** - after
   much trial and error, no combination of signing certificates worked to get a usable product when directly
   signing the binary - but, while `--deep` is often discouraged, in this case, signing the library with exactly
   the same credentials as the application extension is exactly what we want
   * If you build and get a cryptic error about app-id entitlements, you probably have `--deep` set somewhere
   it shouldn't be - your entitlements file gets mucked with by xcodebuild, and its contents are *not* what
   actually gets used at signing time.  The invisible entitlement in question seems to be a munged form of the
   the app-id which is dot-prefixed with your team-id which is generated by the build.
 * Building an app and installer that will not be rejected by Mac OS when downloaded involves *notarizing*
   the app and *stapling* the resulting ticket to the result
   * Notarytool comes with its own huge headaches with regards to generating app-specific credentials (a
     password just for Notarytool you generate on the Apple web site).  It's too much to go into, but
     [this article](https://scriptingosx.com/2021/07/notarize-a-command-line-tool-with-notarytool/) is a useful
     guide containing steps that are up-to-date and don't require non-existent commands to be issued.
     The build script expects a keychain-profile named "notarytool" to provide the credentials to use when
     notarizing an app or installer.
 * Building for installers requires two app-ids registered on the Apple website:
   * A profile named RustDspInProcessDemo for an app id `com.mastfrog.dsp.RustDspInProcessDemo` (or whatever
       you changed it to)
   * A profile named `RustDspInProcessDemoExtension` for an app `com.mastfrog.dsp.RustDspInProcessDemo.RustDspInProcessDemoExtension` (or whatever you changed that to)
     
