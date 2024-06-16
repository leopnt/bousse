# Bousse

A lightweight DJ booth software with trackpad support for personal use

## Why 🤠

Because

- I didn't find a DJ software that gives me a similar experience as having only two turntables and a simple mixer.
- Album covers are very important to find tracks quickly. I want to navigate my music files the same way I look for records.
- I don't have a DJ controller on me right now. It is very expensive, and I wanted to test if a trackpad was sufficient (spoiler: it is).
- I wanted to gain experience building projects in Rust and I thought this was a very complete one (multi-threading, physics simulation, events, UI, audio, DSP, MIDI controller, ...). Fortunately I don't have to go low level as good quality libraries already exist such as `winit`, `kira`, `midir` 🙏. Happens that everything was working as intended, so I just had to glue the parts together and simulate the turntable's behavior.

## NB

This is specifically built for my needs. Some parts are hard-coded, such as the MIDI controller or the trackpad's behavior. Still, it should be easy to adapt anything thanks to the `Controller` interface that I tried to make as universal as possible. It can receive events from any source and dispatch it to the right object (e.g. turntable, mixer).

## Features ⚙️

### MVP Features

This is the features that I consider the bare-minimum to be able to mix tracks together.

- [x] Adjustable channel volumes
- [x] Per-channel, toggle-able cue
- [x] Adjust cue mix between master and cue
- [x] Adjust target pitch of playing sound
- [x] Quick load audio files to a deck via drag & drop
- [x] Display visual feedback such as track progression
- [x] Start and stop a track
- [x] Controllable "vinyl" speed via keyboard and touchpad
  - [x] Playing backward
  - [x] Fast pitch variation
  - [x] Soft touch / temporary pitch shift
  - [x] Hard touch / Cueing
  - [x] Fast seek
- [x] Controllable via MIDI controller

### Additional Features (but still important 🤓)

- [x] Have a debug panel
- [x] Some parts controllable via UI
- [x] Apply EQ filtering to channels
- [x] Dynamic display of album / track covers
- [ ] VU / RMS real time volume meter visual feedback
- [x] File explorer
- [ ] Recording of the master track to a file

### Additional Features (Not required right now)

- [ ] Simple and elegant visualization for current track / mixer state
- [ ] Output to multiple devices

## How to 👨‍💻

Only tested with `Macbook air m1` (nice trackpad) and `Akai MidiMix` hardware.

1. Clone this repo

    ```txt
    git clone [this repo]
    ```

2. Build and execute

    ```txt
    cargo run --release
    ```

    This is important to run in `--release` mode as audio loading is orders of magnitude slower in debug mode.

3. Have fun 🕺💃🪩
