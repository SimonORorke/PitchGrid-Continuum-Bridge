# PitchGrid-Continuum Bridge

PitchGrid-Continuum Bridge is a bridging application that allows tunings specified in [PitchGrid](https://pitchgrid.io/) to tune a [Haken Continuum fingerboard](https://www.hakenaudio.com/).

<img src="images\PitchGrid-Continuum Bridge.png" alt="PitchGrid-Continuum Bridge" style="zoom: 80%;" />

## System Requirements

### Supported Instruments

PitchGrid-Continuum Bridge (PCB) has so far only been tested with the Continuum.  But it should also work with the ContinuuMini, EaganMatrix Eurorack Module and EaganMatrix Micro. For the latter two instruments, connection to an MPE or MIDI keyboard would facilitate use of PitchGrid and PCB, though other configurations may be feasible.  The Osmose is not supported:  although it has the EaganMatrix sound engine in common with those other instruments, it does not provide a public API that would allow tuning.

### Supported Haken Audio Firmware Versions

Custom tuning is broken in Haken Audio Firmware v10.52, the latest production version.  Version 10.73 Beta or later is required.  The most recent Beta version is currently available for anyone to download.

### Supported Operating Systems

PitchGrid-Continuum Bridge has so far only been tested with the Continuum.  But it should also work with macOS and Linux.

### Required Software

The current version of PitchGrid-Continuum Bridge works with PitchGrid 0.33.  PCB will need to be updated to support breaking changes expected in the next version of PitchGrid.  Haken Editor is also required.

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

1. Update the current preset's rounding parameters, if specified in PCB.  (See below.)
2. Save the tuning table to one of the instrument's eight custom tuning grids.
3. Load the custom tuning grid into the current preset.

Once the instrument has implemented the requirements, which should take less than half a second, it sends an acknowledgement back to PCB.  PCB then displays the updated tuning parameters and a confirmation message "Instrument tuning updated".  If all this has worked, Haken Editor will be showing the updated tuning and, if specified, rounding parameters:

<img src="images\Editor Rounding and Tuning.png" alt="Editor Rounding and Tuning" style="zoom: 80%;" />

Whenever a preset is subsequently loaded on the instrument, PCB will update it with the current tuning and, if specified, rounding.

> [!NOTE]
>
> - **Tunings and roundings sent to the instrument are *temporary*.**  That is to say, if the current preset was loaded from a user preset slot, the changes are not saved to the slot.  However, if the instrument is turned off when the current preset's tuning/rounding have been updated, those changes will be in the current preset when the instrument is next turned on.
> - **Real time tuning updates:**  If you sweep one of the tuning controls in PitchGrid, PCB will receive new tunings much faster than the instrument can update and load the tuning table.  Tests have shown that, If PCB were to keep sending updates regardless, the instrument's processor would soon be swamped for minutes!  The solution is to not send more updates to the instrument while another update is in progress and, once the update is complete, send the most recently received following tuning if there is one.

### Tuning Parameters Display

<img src="images\Tuning Parameters - PitchGrid.png" alt="Tuning Parameters - PitchGrid" style="zoom: 80%;" />
<img src="images\Tuning Parameters - PCB.png" alt="Tuning Parameters - PCB" style="zoom: 80%;" />

Once the tuning of the instrument's current preset has been updated in accordance with the tuning parameters received from PitchGrid, PCB displays the applied tuning parameters.  The displayed values may differ from what you can see in PitchGrid in two respects.

- More decimal places are shown.  This is because some PitchGrid tunings cannot be distinguished from each other with the number of decimal places currently shown in PitchGrid, and it is not possible to show the tuning preset name in PCB.
- If a **Root Frequency Override** note is specified in PCB (see below), **Root Freq** will show the overriding note's frequency.

## Preferences

<img src="images\Preferences.png" alt="Preferences" style="zoom: 80%;" />

### Root Frequency Override

