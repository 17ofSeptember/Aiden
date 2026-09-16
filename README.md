# Aiden
<img width="240" height="404" alt="logo" src="https://github.com/user-attachments/assets/59b89194-62c5-446b-a777-9961a9748781" />

**Aiden is a cross-platform biosignal control interface that turns signals from the human body into computer commands.** Its node-based system lets users train biological inputs and connect them to mouse actions, keystrokes, shortcuts, timers, system controls, and more.
<img width="2560" height="1040" alt="workspace" src="https://github.com/user-attachments/assets/6e78fafb-4a6a-428f-b4dc-4e68b0dc4ef7" />

Aiden processes signals locally using deterministic digital signal processing and template matching. It does **not** use AI, machine learning, cloud processing, analytics, or telemetry.

> **Current hardware support:** Aiden has currently only been tested using an **Arduino Uno with an AD8232 Single Lead Heart Rate Monitor** as the biosignal acquisition interface. The architecture is intended to support additional biosensors and acquisition hardware in the future.

## Contents

* [Overview](#overview)
* [Features](#features)
* [How Aiden Works](#how-aiden-works)
* [Current Hardware](#current-hardware)
* [Hardware Connections](#hardware-connections)
* [Installing the Arduino Firmware](#installing-the-arduino-firmware)
* [Installing Aiden on Windows](#installing-aiden-on-windows)
* [Installing Aiden on Linux](#installing-aiden-on-linux)
* [Connecting the Arduino](#connecting-the-arduino)
* [Using Aiden](#using-aiden)
* [Training an Action](#training-an-action)
* [Building Workflows](#building-workflows)
* [Action Profile Modes](#action-profile-modes)
* [Negative Training](#negative-training)
* [Simulation and Test Mode](#simulation-and-test-mode)
* [Example Use Cases](#example-use-cases)
* [Signal Processing](#signal-processing)
* [Safety Features](#safety-features)
* [Saving and Backing Up Data](#saving-and-backing-up-data)
* [Troubleshooting](#troubleshooting)
* [Building Installers](#building-installers)
* [Project Structure](#project-structure)
* [Known Limitations](#known-limitations)
* [Medical Disclaimer](#medical-disclaimer)
* [Author](#author)
* [License](#license)

---

## Overview

Aiden is designed to make biological signals usable as programmable computer inputs.

A biosensor produces a signal representing activity from the body. Aiden receives that signal, processes it in real time, detects repeatable patterns, and lets the user associate those patterns with actions on their computer.

For example, a user could train Aiden to recognize a particular forearm contraction and then configure that contraction to:

* left click
* press a keyboard key
* activate a keyboard shortcut
* hold a key
* move the mouse
* scroll
* adjust volume
* open a program
* open a file or folder
* open a URL
* execute an approved command
* start a timer
* trigger several actions at once

The interface is built around a visual node graph. Inputs, outputs, timers, and logic components can be connected together to create custom control systems without hard-coding each workflow.

Aiden currently focuses on muscle-related biosignals using an AD8232, but the application is designed as a broader **biosignal control interface** rather than a system permanently tied to one sensor.

---

## Features

### Biosignal acquisition

* Arduino Uno serial communication
* AD8232 signal acquisition
* configurable sample rates
* lead-off detection
* connection diagnostics
* dropped sample monitoring
* CRC error monitoring
* live ADC monitoring
* automatic reconnect option

### Real-time signal processing

Aiden performs its signal processing in its native Rust backend rather than in the browser interface.

The processing pipeline includes:

* baseline removal
* configurable high-pass filtering
* configurable low-pass filtering
* optional 50 Hz or 60 Hz notch filtering
* signal rectification
* envelope generation
* baseline noise estimation
* contraction onset detection
* contraction release detection
* event segmentation
* profile matching

### Deterministic action recognition

Aiden does not use machine learning.

Recognition is based on conventional algorithms including:

* normalized waveform comparison
* correlation
* shape distance
* contraction duration
* signal amplitude
* reference templates
* median profile construction
* negative examples
* configurable recognition thresholds
* ambiguity rejection

This makes the recognition system local, inspectable, repeatable, and independent of external services.

### Visual workflow editor

Aiden includes a node-based workspace with:

* drag-and-drop nodes
* visual connections
* multiple outputs from a single input
* node configuration
* undo and redo
* copy and paste
* duplication
* graph event visualization
* configurable event routing

### Computer control

Available node types include:

#### Mouse

* Left Click
* Right Click
* Middle Click
* Mouse Down
* Mouse Up
* Move Mouse
* Vertical Scroll
* Horizontal Scroll

#### Keyboard

* Key Press
* Key Down
* Key Up
* Shortcut
* Type Text

#### System

* Launch Application
* Open File
* Open Folder
* Open URL
* Approved Command

#### Media

* Play/Pause
* Next Track
* Previous Track
* Volume Up
* Volume Down
* Mute

#### Timing

* Delay
* Timer
* Interval
* Debounce
* Cooldown
* Pulse
* Hold
* Repeat

#### Logic

* AND
* OR
* NOT
* Gate
* Toggle
* Latch
* Counter
* Compare
* Branch
* Debug

---

# How Aiden Works

The basic signal path is:

```text
Human movement
      |
      v
Biosensor
      |
      v
Arduino Uno
      |
      v
USB Serial
      |
      v
Aiden
      |
      v
Signal Processing
      |
      v
Action Recognition
      |
      v
Node Graph
      |
      v
Computer Action
```

Aiden first establishes a baseline while the user is relaxed.

When activity rises above the configured detection threshold, Aiden begins capturing the signal.

When the signal returns below the release threshold, the contraction is converted into an action candidate.

The candidate is compared against trained Action Profiles.

If the candidate:

1. matches a trained profile strongly enough,
2. passes that profile's recognition threshold,
3. is sufficiently different from competing profiles,
4. is not too similar to a negative example,

the corresponding Action Profile is triggered.

The resulting event is then sent through the node graph.

---

# Current Hardware

The current tested hardware configuration is:

* Arduino Uno
* AD8232 Single Lead Heart Rate Monitor
* three compatible electrodes
* USB data cable
* Windows or Linux computer

> **Important:** Aiden has currently only been tested with the Arduino Uno + AD8232 configuration described in this README.

Other biosensors may eventually be supported, but their compatibility should not currently be assumed.

---

# Hardware Connections

The included Arduino firmware expects the following connections.

| AD8232   | Arduino Uno     |
| -------- | --------------- |
| `3.3V`   | `3.3V`          |
| `GND`    | `GND`           |
| `OUTPUT` | `A0`            |
| `LO+`    | Digital Pin `8` |
| `LO-`    | Digital Pin `9` |

The firmware currently defines:

```cpp
constexpr uint8_t ADC_CHANNEL = 0;
constexpr uint8_t LEAD_PLUS_PIN = 8;
constexpr uint8_t LEAD_MINUS_PIN = 9;
```

The AD8232 should be powered from the Arduino's **3.3 V output**, not the 5 V pin.

## Electrode placement

Aiden does not require medical ECG electrode positioning.

For muscle experiments, the general goal is to place the electrodes so the AD8232 receives a clear and repeatable change when the target movement occurs.

A useful starting configuration is:

```text
Electrode 1
     |
     |  Target muscle
     |
Electrode 2

Reference electrode
placed on a nearby quieter area
```

For example, when experimenting with finger flexion, begin with the primary sensing electrodes along the forearm muscle area involved in the finger movement and place the reference electrode on a nearby electrically quieter location.

Electrode placement has a significant effect on recognition.

Move the electrodes gradually and use Aiden's **Signal** page to observe the waveform before training a profile.

A good placement should produce:

* relatively stable signal while relaxed
* clearly visible activity during the intended movement
* repeatable signal shapes
* minimal accidental activity from unrelated movement

Different people and different muscles may require different placement.

Record useful electrode positions in the Action Profile's **Electrode placement notes** field.

---

# Installing the Arduino Firmware

The firmware is located in:

```text
firmware/arduino-uno.ino
```

It performs signal acquisition only.

All signal processing, training, recognition, and computer control occur on the PC.

## Using Arduino IDE

### 1. Install Arduino IDE

Install Arduino IDE 2 if it is not already installed.

### 2. Connect the Uno

Connect the Arduino Uno to your computer using a USB **data** cable.

### 3. Open the firmware

Open:

```text
firmware/arduino-uno.ino
```

in Arduino IDE.

### 4. Select the board

Select:

```text
Arduino Uno
```

as the target board.

### 5. Select the serial port

Choose the serial port corresponding to the Uno.

Windows typically uses a name such as:

```text
COM3
COM4
COM5
```

Linux typically uses something similar to:

```text
/dev/ttyACM0
```

or:

```text
/dev/ttyUSB0
```

### 6. Upload

Compile and upload the sketch.

No external Arduino libraries are required by the included firmware.

### 7. Close Serial Monitor

Do not leave Arduino Serial Monitor open while using Aiden.

Only one program can normally control the Arduino serial port at a time.

---

# Installing Aiden on Windows

Aiden is a Tauri desktop application with:

* Rust native backend
* React + TypeScript frontend
* native Windows input control
* SQLite local persistence

## Requirements

Install:

* Node.js 22
* npm
* Rust
* Microsoft Visual C++ Build Tools
* Windows SDK
* Microsoft Edge WebView2 Runtime

When installing Visual Studio Build Tools, enable:

```text
Desktop development with C++
```

Aiden uses the Rust MSVC toolchain on Windows.

## 1. Open PowerShell

Navigate to the Aiden source folder:

```powershell
cd C:\path\to\Aiden
```

For example:

```powershell
cd C:\Users\YourName\Desktop\Aiden
```

## 2. Install JavaScript dependencies

```powershell
npm.cmd ci
```

Using `npm.cmd` avoids problems on systems where PowerShell prevents execution of `npm.ps1`.

## 3. Confirm Rust

```powershell
rustc --version
cargo --version
```

If both commands work, Rust is available.

## 4. Run Aiden

For development/source use:

```powershell
npm.cmd run tauri -- dev
```

Aiden should compile and open as a native desktop application.

You do not need to manually open the Vite development URL in a browser.

## Building the Windows installer

To create an installable Windows package:

```powershell
$env:CARGO_BUILD_JOBS = '1'
npm.cmd run tauri -- build --bundles nsis
```

The finished installer is created under:

```text
src-tauri\target\release\bundle\nsis\
```

The installer produced from the current source is unsigned.

---

# Installing Aiden on Linux

Aiden currently supports native computer-control output on **X11**.

Wayland global input injection is intentionally not treated as supported by the current implementation.

The Linux instructions were designed primarily around Linux Mint / Ubuntu-based distributions.

## Install system dependencies

```bash
sudo apt update

sudo apt install -y \
  build-essential \
  pkg-config \
  curl \
  ca-certificates \
  git \
  libwebkit2gtk-4.1-dev \
  libudev-dev \
  libxdo-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  xdg-utils
```

## Install Node.js 22

Aiden currently targets Node.js 22 for source builds.

Using `nvm` is one option.

After installation, verify:

```bash
node --version
npm --version
```

## Install Rust

Install the current stable Rust toolchain using `rustup`.

Verify:

```bash
rustc --version
cargo --version
```

## Install Aiden dependencies

From the project directory:

```bash
npm ci
```

## Run Aiden from source

```bash
npm run tauri -- dev
```

## Build a Debian package

```bash
CARGO_BUILD_JOBS=1 npm run tauri -- build --bundles deb
```

The package is created under:

```text
src-tauri/target/release/bundle/deb/
```

Install it with:

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/*.deb
```

Aiden can then be launched from the application menu or with:

```bash
aiden
```

## X11 requirement

Check the current session:

```bash
echo "$XDG_SESSION_TYPE"
```

For native computer control, the current version of Aiden expects:

```text
x11
```

If the result is:

```text
wayland
```

the signal-processing and application portions may still function, but Aiden will not enable unsupported global input injection.

Use an X11 desktop session when using Aiden for computer control.

## Arduino permissions on Linux

Add your user to the serial-device group:

```bash
sudo usermod -aG dialout "$USER"
```

Then completely log out and log back in.

Check membership:

```bash
id -nG
```

You should see:

```text
dialout
```

Do not run Aiden as root simply to access the Arduino.

---

# Connecting the Arduino

After installing the firmware:

1. Connect the Arduino Uno.
2. Start Aiden.
3. Open **Device**.
4. Select **Arduino Uno / AD8232**.
5. Click **Refresh ports**.
6. Select the Arduino's serial port.
7. Select the baud rate configured by `SERIAL_BAUD` in the firmware.
8. Click **Connect**.

Aiden's diagnostics should begin showing:

* Connected status
* Firmware version
* Protocol version
* sample rate
* increasing sample count
* dropped samples
* CRC errors
* lead status
* live ADC value
* baseline noise
* baseline readiness

The default signal-processing sample rate is:

```text
500 Hz
```

The firmware supports:

```text
250 Hz
500 Hz
1000 Hz
```

500 Hz is the recommended starting point.

Higher sampling rates also require sufficient serial bandwidth.

---

# Using Aiden

Aiden is divided into several primary areas.

## Workspace

The main node-based automation editor.

Use it to connect trained biosignal inputs to computer actions.

## Device

Used to:

* connect hardware
* choose simulation modes
* choose serial ports
* view connection diagnostics

## Signal

Displays the live waveform.

Available traces include:

* raw
* filtered
* rectified
* envelope

Use this screen when finding good electrode placement.

## Training

Creates and manages Action Profiles.

## Settings

Contains:

* DSP configuration
* workspace export/import
* profile import
* database backup
* raw recording controls
* recording playback

## Diagnostics

Displays internal device, processing, and graph runtime information.

## About

Displays project information and version details.

---

# Training an Action
<img width="2560" height="1038" alt="training" src="https://github.com/user-attachments/assets/26eca572-466f-4669-9762-c9599ac136c8" />

An **Action Profile** represents a biological signal that Aiden should recognize.

For example:

```text
Index Finger Flex
```

or:

```text
Forearm Contraction
```

## Step 1: Find the signal

Connect the Arduino and open the **Signal** page.

Relax the target muscle.

Perform the intended movement several times.

Look for a clear and repeatable change in the filtered signal and envelope.

If the movement is difficult to distinguish from rest, adjust electrode placement before training.

## Step 2: Create a profile

Open **Training**.

Enter a name such as:

```text
Index Finger Flex
```

Click:

```text
Create profile
```

## Step 3: Capture the reference

Relax completely.

Click:

```text
Read reference
```

Aiden allows approximately three seconds for baseline preparation before collecting the reference activity.

Perform the desired movement once and release.

Then click:

```text
Done
```

Aiden searches the captured data for the contraction and extracts the candidate segment.

## Step 4: Review the reference

Review the captured waveform.

If the automatically selected segment is not correct, adjust the start and end samples.

When satisfied, select:

```text
Accept candidate
```

This becomes a reference example for the profile.

## Step 5: Train repetitions

Click:

```text
Train repetitions
```

Perform the same movement repeatedly.

Aiden continuously watches the signal and compares detected contractions against the existing profile.

Strong matches are automatically accepted as additional examples.

The training interface displays information such as:

* accepted examples
* rejected examples
* template similarity
* waveform
* signal variant

Continue until the movement is being detected consistently.

More examples can help represent natural differences between repetitions, but poor or inconsistent examples can reduce profile quality.

Quality is more important than simply collecting a large number of repetitions.

## Step 6: Stop training

When finished, pause or stop the training process.

Review the saved examples.

Bad examples can be deleted individually.

## Step 7: Save the profile

Configure:

* Name
* Description
* Electrode placement notes
* Enabled state
* Trigger mode
* Match threshold
* Ambiguity margin
* Sensitivity
* Cooldown
* Minimum duration
* Maximum duration

Then click:

```text
Save profile settings
```

---

# Building Workflows

Once an Action Profile has been trained, return to **Workspace**.

A basic workflow might look like:

```text
[Index Finger Flex]
          |
          v
   [Left Click]
```

Connect the output port of the Action node to the input of the Left Click node.

When computer control is enabled and Aiden recognizes the trained signal, the graph triggers the mouse action.

## Multiple outputs

One biological input can control multiple nodes.

Example:

```text
                  +--> [Left Click]
                  |
[Forearm Flex] ---+--> [Delay 500 ms] --> [Key Press: F]
                  |
                  +--> [Debug]
```

A single detected event can therefore start several independent workflows.

---

# Action Profile Modes

Aiden supports three profile modes.

## Discrete

One completed contraction produces one trigger.

Example:

```text
Finger Flex
    |
    v
Left Click
```

This is the recommended mode for actions such as:

* clicking
* launching applications
* shortcuts
* single key presses
* toggles

A sustained contraction does not intentionally generate hundreds of click events.

## Hold

Provides start and release behavior.

This can be useful when a physical contraction should behave more like holding a button.

Example concept:

```text
Muscle START
    |
    v
Key Down

Muscle END
    |
    v
Key Up
```

## Continuous

Provides repeated active events and signal strength information while the contraction remains active.

This is useful for controls such as:

* sustained mouse movement
* scrolling
* variable movement speed

---

# Negative Training

Two movements can sometimes produce similar electrical signals, particularly when measured through a single sensor channel.

Aiden includes negative training to help reduce false activation.

Open the profile's advanced training controls and select:

```text
Train non-target movements
```

Then perform movements that should **not** activate the profile.

Examples might include:

* moving an adjacent finger
* relaxing
* rotating the wrist
* flexing a different forearm muscle

These samples are stored as negative examples.

When a future candidate resembles the negative examples too closely, Aiden rejects the activation.

This can be especially useful when creating several Action Profiles from nearby muscles.

---

# Signal Variants

A single physical action can sometimes appear differently because of:

* electrode position
* contraction strength
* arm position
* posture
* minor placement changes

Aiden supports multiple named signal variants inside an Action Profile.

For example:

```text
Index Finger
    Default
    Arm Resting
    Arm Raised
```

These remain part of the same logical action while providing several deterministic references for recognition.

---

# Simulation and Test Mode

Aiden includes signal simulation so the application can be explored without physical hardware.

Available simulated sources include:

* periodic contraction
* sine signal
* baseline noise

Recorded signals can also be replayed.

## Test Graph

Click:

```text
Test Graph
```

to evaluate graph behavior without allowing real operating-system input.

The top bar displays:

```text
TEST GRAPH · NO OS INPUT
```

This mode is useful when building or debugging workflows.

## Live control

Actual computer control requires a hardware source.

Click:

```text
Enable control
```

and confirm the safety prompt.

The top bar changes to:

```text
COMPUTER CONTROL ON
```

Only then can the graph perform actual computer actions.

---

# Example Use Cases

Aiden is intentionally general-purpose. The following examples demonstrate some of the ways its biosignal input system can be used.

## 1. Finger flex as left click

Train:

```text
Index Finger Flex
```

Build:

```text
[Index Finger Flex]
          |
          v
    [Left Click]
```

Each recognized flex performs one mouse click.

---

## 2. Muscle shortcut controller

Train:

```text
Forearm Contraction
```

Connect it to:

```text
[Forearm Contraction]
          |
          v
 [Shortcut: Ctrl+Shift+S]
```

The trained contraction can activate an application shortcut.

---

## 3. Timed multi-action sequence

A biosignal can trigger a sequence rather than just one command.

Example:

```text
[Muscle Action]
      |
      v
 [Key Press]
      |
      v
 [Delay 500 ms]
      |
      v
 [Key Press]
```

This can create physical shortcuts for multi-step computer operations.

---

## 4. One movement, several actions

Graph fan-out allows one signal to activate several branches:

```text
                   +--> [Key Press]
                   |
[Muscle Action] ---+--> [Timer] --> [Mouse Click]
                   |
                   +--> [Debug]
```

---

## 5. Muscle-controlled mouse movement

Separate Action Profiles could be trained for:

```text
Move Left
Move Right
Move Up
Move Down
```

and connected to corresponding **Move Mouse** nodes.

For example:

```text
[Left Muscle Action]
        |
        v
[Move Mouse: Left]
```

Continuous mode can be used for sustained movement.

---

## 6. Accessibility experiments

Aiden can be used experimentally to create alternative computer interfaces for people who may find conventional keyboards or mice difficult to operate.

Possible mappings include:

```text
Muscle contraction -> Mouse click
Muscle contraction -> Enter
Muscle contraction -> Space
Muscle contraction -> Application shortcut
```

Aiden is not certified medical or assistive technology, so any accessibility use should be treated as experimental.

---

## 7. Media control

A biosignal could control media playback:

```text
[Action A] --> [Play/Pause]

[Action B] --> [Next Track]

[Action C] --> [Volume Up]
```

---

## 8. Application launcher

A trained contraction can launch an approved local program.

```text
[Muscle Action]
      |
      v
[Launch Application]
```

System targets require explicit approval for the current Aiden session before execution.

---

## 9. Custom logic

Logic nodes can create more complicated behavior.

Example:

```text
[Action A] --\
              [AND] --> [Output]
[Action B] --/
```

Other available logic tools include:

* OR
* NOT
* Toggle
* Latch
* Counter
* Gate
* Compare
* Branch

---

# Signal Processing

Aiden currently uses a configurable deterministic processing pipeline.

Conceptually:

```text
ADC Signal
    |
    v
Baseline Removal
    |
    v
High-Pass Filter
    |
    v
Optional 50/60 Hz Notch
    |
    v
Low-Pass Filter
    |
    v
Rectification
    |
    v
Smoothed Envelope
    |
    v
Onset / Release Detection
    |
    v
Segment Extraction
    |
    v
Template Comparison
```

The default DSP configuration includes:

```text
Sample rate:       500 Hz
High-pass:           5 Hz
Low-pass:          100 Hz
Notch:         Disabled
Envelope:           35 ms
Baseline:         1500 ms
Minimum event:     100 ms
Maximum event:    5000 ms
Cooldown:          350 ms
```

These values are adjustable in **Settings**.
<img width="2560" height="1036" alt="settings" src="https://github.com/user-attachments/assets/678f92e3-5600-4d5f-9f99-80a0ba7621bf" />


Changing signal-processing settings may invalidate existing trained profiles because the signal representation changes.

Retraining after significant DSP changes is recommended.

---

# How Recognition Works

Aiden's training system is deterministic.

Each captured contraction is transformed into a normalized signal representation.

The current recognition implementation considers characteristics including:

* normalized signal shape
* waveform correlation
* shape distance
* contraction duration
* amplitude
* profile sensitivity

Multiple accepted examples are combined using robust median-based references.

A candidate must pass the selected profile's threshold.

If another enabled profile produces a score too close to the best result, the candidate is rejected according to the profile's:

```text
Ambiguity Margin
```

This helps prevent similar Action Profiles from triggering each other.

Negative examples provide another rejection mechanism.

There is no neural network, model training, LLM, or cloud classifier involved.

---

# Safety Features

Because Aiden can control the keyboard, mouse, applications, and system actions, several safeguards are included.

## Monitoring starts disabled

Starting Aiden does not automatically activate computer control.

## Test Graph mode

Graph behavior can be tested without executing OS actions.

## Explicit live-control confirmation

The user must explicitly enable real computer control.

## Emergency stop

The application includes an:

```text
Emergency stop
```

button.

A global emergency shortcut is also available:

```text
Ctrl + Shift + F12
```

This stops control and releases held input states.

Test the emergency stop before experimenting with complex or sustained workflows.

## System action approval

Sensitive nodes such as:

* Launch Application
* Approved Command
* Open File
* Open Folder
* Open URL

require the configured target to be reviewed and approved for the current session.

---

# Saving and Backing Up Data

Aiden stores its local state in SQLite.

Data can include:

* workspaces
* graph nodes
* graph connections
* Action Profiles
* training examples
* validation information
* recordings
* settings

## Export workspace

Open:

```text
Settings -> Export workspace
```

to create a portable workspace JSON file.

## Import workspace

Use:

```text
Settings -> Import / restore workspace
```

Importing a workspace replaces the current workspace.

Back up anything important first.

## Export individual profile

A profile can also be exported independently.

This is useful for:

* backups
* transferring a profile
* experimenting with settings

Profiles are highly dependent on electrode placement and the individual who recorded them.

A profile created by one person should not be assumed to work for another person.

## Database backup

Aiden also provides a local database backup function under Settings.

---

# Raw Signal Recording

Aiden can record incoming raw signal sessions.

Open **Settings** and use:

```text
Record signal
```

When finished:

```text
Stop and save recording
```

Saved recordings can be replayed through Aiden's processing pipeline in Test mode.

This is useful for:

* debugging
* recognition testing
* comparing DSP settings
* developing workflows without repeatedly performing the physical movement

---

# Troubleshooting

## Arduino does not appear

Check:

* USB cable supports data
* Arduino is powered
* correct USB port is being used
* Arduino drivers are installed
* Arduino Serial Monitor is closed
* no other program owns the serial port

Then click:

```text
Refresh ports
```

---

## Connected but no samples appear

Check:

* Aiden firmware is installed
* selected serial port is correct
* Aiden baud rate matches `SERIAL_BAUD` in the firmware
* USB cable is stable
* Arduino is not connected to another serial application

Use the Device diagnostics to watch:

```text
Samples
Dropped samples
CRC errors
```

---

## Signal remains flat

Check:

* AD8232 has 3.3 V
* AD8232 ground is connected
* OUTPUT is connected to A0
* electrodes are securely attached
* electrode cable is properly inserted
* electrode placement reaches the target muscle

Open **Signal** and perform a stronger contraction while observing the live waveform.

---

## Baseline never becomes ready

Remain relaxed for several seconds.

Also check:

* excessive noise
* ADC clipping
* loose electrodes
* lead-off status
* poor electrode placement

---

## Too many false triggers

Try:

1. collecting more high-quality reference examples
2. adding negative examples
3. increasing Match Threshold
4. increasing Ambiguity Margin
5. increasing Cooldown
6. improving electrode placement
7. reducing unrelated movement
8. separating very similar Action Profiles

---

## Correct movement is not detected

Try:

1. reviewing the Signal page
2. confirming the movement still resembles the trained examples
3. improving electrode contact
4. adding additional training repetitions
5. adjusting sensitivity
6. slightly reducing the match threshold
7. checking minimum and maximum duration
8. retraining after moving the electrodes

---

## Linux serial permission denied

Run:

```bash
sudo usermod -aG dialout "$USER"
```

Then log out and back in.

Check:

```bash
id -nG
```

---

## Linux computer control unavailable

Check:

```bash
echo "$XDG_SESSION_TYPE"
```

Aiden's current Linux global input backend requires:

```text
x11
```

Wayland global input injection is not currently supported.

---

## Movement is detected but nothing happens

Confirm:

* the Action Profile is enabled
* the node is connected to an output
* the correct graph event is selected
* the top bar says `COMPUTER CONTROL ON`
* system targets have been approved
* output settings are correct

Remember that editing or saving portions of the workspace can intentionally stop monitoring for safety.

---

# Development and Validation

Useful project commands include:

```bash
npm run lint
npm test
npm run build
```

Rust validation:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

cargo clippy --locked \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets \
  -- -D warnings

cargo test --locked \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets
```

Compile the Rust application:

```bash
cargo build --locked --manifest-path src-tauri/Cargo.toml
```

---

# Building Installers

## Windows NSIS

From PowerShell:

```powershell
$env:CARGO_BUILD_JOBS = '1'
npm.cmd run tauri -- build --bundles nsis
```

Output:

```text
src-tauri\target\release\bundle\nsis\
```

## Linux DEB

```bash
CARGO_BUILD_JOBS=1 npm run tauri -- build --bundles deb
```

Output:

```text
src-tauri/target/release/bundle/deb/
```

## Linux AppImage

```bash
CARGO_BUILD_JOBS=1 npm run tauri -- build --bundles appimage
```

Output:

```text
src-tauri/target/release/bundle/appimage/
```

---

# Project Structure

A simplified project layout:

```text
Aiden/
├── firmware/
│   └── arduino-uno.ino
│
├── fixtures/
│   └── simulation-workspace.json
│
├── public/
│   └── logo.png
│
├── src/
│   ├── components/
│   ├── graph/
│   ├── App.tsx
│   ├── api.ts
│   ├── main.tsx
│   ├── style.css
│   ├── types.ts
│   └── useWorkspace.ts
│
├── src-tauri/
│   ├── src/
│   │   ├── actions.rs
│   │   ├── engine.rs
│   │   ├── graph.rs
│   │   ├── model.rs
│   │   ├── recognition.rs
│   │   ├── serial.rs
│   │   ├── signal.rs
│   │   ├── simulation.rs
│   │   ├── storage.rs
│   │   └── training.rs
│   │
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── tauri.conf.json
│
├── package.json
├── package-lock.json
├── tsconfig.json
├── vite.config.ts
├── README.md
└── LICENSE
```

---

# Technology

Aiden is primarily built with:

### Native backend

* Rust
* Tauri 2
* SQLite
* serial communication
* native operating-system input APIs

### Frontend

* React
* TypeScript
* Vite
* React Flow

### Firmware

* Arduino C/C++
* Arduino Uno

The native Rust backend handles the timing-sensitive portions of the application, including:

* serial acquisition
* signal processing
* recognition
* graph execution
* persistence
* computer-control dispatch

The React interface is primarily responsible for visualization and configuration.

---

# Known Limitations

### One biosignal channel

The current AD8232 implementation provides one biosignal channel.

Separating very similar muscle movements can therefore be difficult.

Future hardware with multiple channels may provide considerably more information.

### Electrode placement matters

Moving an electrode can alter the signal enough to reduce recognition reliability.

Retraining may be necessary after placement changes.

### AD8232 limitations

The AD8232 is primarily designed as a biopotential/ECG front end rather than a dedicated professional EMG acquisition system.

Aiden can use the electrical activity it provides for experimental human-computer interaction, but it should not be assumed to offer the performance of dedicated multi-channel EMG hardware.

### Linux Wayland

The current Linux computer-control backend targets X11.

Wayland global input injection is not currently supported.

### macOS

The current project targets Windows and Linux.

macOS support should not currently be assumed.

### Current hardware testing

The current release has only been tested with:

```text
Arduino Uno + AD8232
```

Compatibility with other microcontrollers, ADCs, EMG sensors, EEG devices, or other biosignal hardware has not yet been established.

---

# Future Hardware

Aiden is intended to eventually support more than muscle signals.

Potential future signal sources could include:

* dedicated EMG sensors
* multi-channel EMG systems
* EEG interfaces
* EOG sensors
* pressure and force sensors
* motion sensors
* other biopotential hardware
* custom microcontroller interfaces

The long-term goal is for Action Profiles to represent biological or physical inputs independently from the specific sensor used to acquire them.

---

# Privacy

Aiden is designed to operate locally.

The current application does not require:

* cloud processing
* online accounts
* analytics
* telemetry
* AI services
* remote signal processing

Biosignal recordings and trained profiles remain on the local machine unless the user explicitly exports or shares them.

---

# Medical Disclaimer

**Aiden is not a medical device.**

It is not intended for:

* diagnosis
* treatment
* patient monitoring
* medical decision-making
* measuring or diagnosing health conditions

The AD8232 is used by Aiden only as a signal acquisition interface for experimental human-computer interaction.

Waveforms, signal strength, electrode status, and other information displayed by Aiden must not be interpreted as medical information.

---

# Contributing

Contributions, testing, hardware experiments, bug reports, and improvements are welcome.

When contributing, please keep Aiden's core principles in mind:

* local processing
* deterministic recognition
* user-controlled automation
* no required cloud services
* no required AI or machine learning
* safe computer-control behavior
* clear hardware limitations
* cross-platform architecture

If adding support for additional biosensors or acquisition devices, avoid tightly coupling the rest of the application to that specific hardware.

---

# Author

**Aiden was created by Sam (17ofSeptember).**

GitHub:

https://github.com/17ofSeptember

Support development:

https://buymeacoffee.com/17ofseptember

Contact:

[awrynetwork@gmail.com](mailto:awrynetwork@gmail.com)

---

# License

Aiden is distributed under the license included in the repository's `LICENSE` file.

Copyright © 2026 Sam (17ofSeptember).

See `LICENSE` for the complete terms.

---

## Support Aiden

If Aiden is useful to you and you would like to support continued development:

https://buymeacoffee.com/17ofseptember

Project author:

**Sam (17ofSeptember)**
https://github.com/17ofSeptember
[awrynetwork@gmail.com](mailto:awrynetwork@gmail.com)
