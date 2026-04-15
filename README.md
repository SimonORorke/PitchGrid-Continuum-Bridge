# PitchGrid-Continuum Bridge

PitchGrid-Continuum Bridge is a bridging application that allows tunings specified in [PitchGrid](https://pitchgrid.io/) to tune a [Haken Continuum fingerboard](https://www.hakenaudio.com/).

<img src="images\PitchGrid-Continuum Bridge.png" alt="PitchGrid-Continuum Bridge" style="zoom: 80%;" />

## System Requirements

### Supported Instruments

PitchGrid-Continuum Bridge (PCB) has so far only been tested with the Continuum.  But it should also work with the ContinuuMini, EaganMatrix Eurorack Module and EaganMatrix Micro. For the latter two instruments, connection to an MPE or MIDI keyboard would facilitate use of PitchGrid and PCB, though other configurations may be feasible.  The Osmose is not supported:  although it has the EaganMatrix sound engine in common with those other instruments, it does not provide a public API that would allow tuning.

### Supported Haken Audio Firmware Versions

Custom tuning is broken in Haken Audio Firmware v10.52, the latest production version.  Version 10.72 Beta or later is required.  The most recent Beta version is currently available for anyone to download.

### Supported Operating Systems

PitchGrid-Continuum Bridge has so far only been tested with the Continuum.  But it should also work with macOS and Linux.

### Required Software

PitchGrid 0.3.4 or later
Haken Editor

## Connections

<img src="images\Data Flow.jpg" alt="Data Flow" style="zoom: 100%;" />

For use with PitchGrid-Continuum Bridge and a Continuum, PitchGrid has no input and sends tuning data via OSC to PCB. PCB sends only heartbeat messages every second to PitchGrid, indicating that tuning updates are required. **Sync Tuning Data via OSC** *must be enabled in PitchGrid's Output menu.*

<img src="images\PitchGrid OSC Enabled.png" alt="PitchGrid OSC Enabled" style="zoom: 80%;" />

PCB connects to Haken Editor's External input and output in All Data mode. As usual, Haken Editor's instrument input and output connect to the instrument.

<img src="images\PCB MIDI Connections.png" alt="PCB MIDI Connections" style="zoom: 80%;" />
<img src="images\Editor MIDI Settings.png" alt="Editor MIDI Settings" style="zoom: 80%;" />

> [!WARNING]
>
> *PitchGrid-Continuum Bridge must not be connected directly to the instrument.  Doing that causes a MIDI loop, which is indicated on the instrument's display.*
>
> <img src="images\Loop.png" alt="Loop" style="zoom: 100%;" />

### Connecting to Haken Editor

As PitchGrid-Continuum Bridge and Haken Editor are both software, you (obviously) cannot connect them with cables.  Instead, these options are available:

- Virtual MIDI ports.

- Loopback endpoints, which in Windows are provided by Microsoft's recently introduced Windows MIDI Services.
- A USB MIDI router, such as [IConnectivity's mioXL and mioXM](https://www.iconnectivity.com/midi-interfaces-1).

### Load Order

The order in which you load or turn on or connect the components does not matter.  PitchGrid-Continuum Bridge will show messages advising you of anything that is not yet connected.
<img src="images\Awaiting PitchGrid Connection.png" alt="Awaiting PitchGrid Connection" style="zoom: 80%;" />

## Tuning

On first connecting PitchGrid to PCB and whenever you change the tuning, PitchGrid sends the tuning parameters to PCB. PCB converts the PitchGrid tuning parameters to a 128-key tuning table and sends the following instructions to the instrument.

1. Override the current preset's rounding parameters, if specified in PCB.  (See below.)
2. Save the tuning table to one of the instrument's eight custom tuning grids.
3. Load the custom tuning grid into the current preset.

Once the instrument has implemented the requirements, which should take less than half a second, it sends an acknowledgement back to PCB.  PCB then displays the updated tuning parameters and a confirmation message "Instrument tuning updated".  If all this has worked, Haken Editor will be showing the updated tuning and, if overriden in PCB, rounding parameters:

<img src="images\Editor Rounding and Tuning.png" alt="Editor Rounding and Tuning" style="zoom: 80%;" />

Whenever a preset is subsequently loaded on the instrument, PCB will update it with the current tuning and, if specified, rounding.

> [!NOTE]
>
> - **Tunings and roundings sent to the instrument are *temporary*.**  That is to say, if the current preset was loaded from a user preset slot, the changes are not saved to the slot.  However, if the instrument is turned off when the current preset's tuning/rounding have been updated, those changes will be in the current preset when the instrument is next turned on.
> - **Real time tuning updates:**  If you sweep one of the tuning controls in PitchGrid, PCB will receive new tunings much faster than the instrument can update and load the tuning table.  Tests have shown that, If PCB were to keep sending updates regardless, the instrument's processor would soon be swamped for minutes!  The solution is to not send more updates to the instrument while another update is in progress and, once the update is complete, send the most recently received following tuning if there is one.

### Tuning Parameters Display

<img src="images\Tuning Parameters - PitchGrid.png" alt="Tuning Parameters - PitchGrid" style="zoom: 80%;" />
<img src="images\Tuning Parameters - PCB.png" alt="Tuning Parameters - PCB" style="zoom: 80%;" />

Once the tuning of the instrument's current preset has been updated in accordance with the tuning parameters received from PitchGrid, PCB displays the applied tuning parameters.  Instead of Depth, PCB receives and displays the MOS system's counts of large and small steps, which vary with Depth. The displayed values may differ from what you can see in PitchGrid in two respects.

- More decimal places are shown.  This is because some PitchGrid tunings cannot be distinguished from each other with the number of decimal places currently shown in PitchGrid, and it is not possible to show the tuning preset name in PCB.
- If a **Root Frequency Override** note is specified in PCB (see below), **Root Freq** will show the overriding note's frequency.

## Options

<img src="images\Options.png" alt="Options" style="zoom: 80%;" />

### Root Frequency Override

<img src="images\Root Freq PitchGrid.png" alt="Root Freq PitchGrid" style="zoom: 80%;" /><img src="images\Root Freq Override.png" alt="Root Freq Override" style="zoom: 80%;" /><img src="images\Root Freq PCB.png" alt="Root Freq PCB" style="zoom: 80%;" />

The Root Frequency specified in PitchGrid, which is Middle C for most tuning presets, may be overriden. A 12-TET note from the F# below Middle C to the F above Middle C may be selected. These notes are in concert pitch; so if A is selected,  the overriding frequency will be 220 Hz. If an override is not required, the blank item should be selected.  If the override is changed when the instrument's current preset has already been tuned, the tuning will be sent again with the overriding root frequency.

When a root frequency has been overridden, the overriding frequency will be shown in PitchGrid-Continuum Bridge's tuning parameters display. By design, Root Freq Override is not saved to PCB's settings: the assumption is that, for safety, the player should consider which override, if any, to use each time PCB is loaded.

### Pitch Table

<img src="images\Pitch Table.png" alt="Pitch Table" style="zoom: 80%;" />The identifier of the pitch table to which the tuning is to be uploaded may be selected from the range 80 to 87, which the Haken firmware reserves for custom tuning grids.  Unless you will be using the instrument's custom tuning grids for purposes other than receiving PitchGrid tunings via PitchGrid-Continuum Bridge, you can safely leave this to the default, 80.

### Rounding Overrides

<img src="images\Rounding Overrides.png" alt="Rounding Overrides" style="zoom: 80%;" />

With microtonal/xenharmonic tunings. it may be useful to constrain (to a greater or lesser extent) the fingerboard to play the pitches specified in the tuning table. So when a tuning is sent to the instrument, the preset's rounding parameters may optionally be overridden.

If Rounding override **Initial** is On, rounds each note's initial pitch to the key's specified  tuning pitch; otherwise the preset's Initial Rounding parameter is unchanged.

If Rounding override **Rate** is On, sets Rounding Mode to Normal with the specified **Rounding Rate** value; otherwise the preset's Rounding Mode and Rounding Rate parameters are unchanged.

Rounding override **Rate** On with **Rounding Rate** 127 (the maximum) effectively enforces initial rounding, even when the preset's Initial Rounding parameter is Off. In addition, it prevents  the pitch from being changed by subsequent motion of the finger on the fingerboard.
