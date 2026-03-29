# PitchGrid-Continuum Bridge

PitchGrid-Continuum Bridge is a bridging application that allows tunings specified in [PitchGrid](https://pitchgrid.io/) to tune a [Haken Continuum fingerboard](https://www.hakenaudio.com/).

<img src="images\PitchGrid-Continuum Bridge.png" alt="PitchGrid-Continuum Bridge" style="zoom: 80%;" />

### Supported Instruments

PitchGrid-Continuum Bridge (PCB) has so far only been tested with the Continuum.  But it should also work with the ContinuuMini, EaganMatrix Eurorack Module and EaganMatrix Micro. For the latter two instruments, connection to an MPE or MIDI keyboard would facilitate use of PitchGrid and PCB, though other configurations may be possible.  The Osmose is not supported:  although it has the EaganMatrix sound engine in common with those other instruments, it does not provide a public API that would support tuning.

### Supported Haken Audio Firmware Versions

Custom tuning is broken in Haken Audio Firmware v10.52, the latest production version.  It is fixed in the subsequent beta versions, the most recent of which is available for anyone to download.

### Supported Operating Systems

PitchGrid-Continuum Bridge has so far only been tested with the Continuum.  But it should also work with macOS and Linux.

### Required Software

The current version of PitchGrid-Continuum Bridge works with PitchGrid 0.33.  PCB will need to be updated to support breaking changes expected in the next version of PitchGrid.  Haken Editor is also required.

### Connections

<img src="images\Data Flow.jpg" alt="Data Flow" style="zoom: 100%;" />

For use with PitchGrid-Continuum Bridge and a Continuum, PitchGrid has no input and sends tuning data via OSC to PCB. PCB sends only heartbeat messages every second to PitchGrid, indicating that tuning updates are required. **Sync Tuning Data via OSC** *must be enabled in PitchGrid's Output menu.*

<img src="images\PitchGrid OSC Enabled.png" alt="PitchGrid OSC Enabled" style="zoom: 80%;" />

PCB connects to Haken Editor's External input and output in All Data mode. As usual, Haken Editor's instrument input and output connect to the instrument.

<img src="images\PCB MIDI Connections.png" alt="PCB MIDI Connections" style="zoom: 80%;" />
<img src="images\Editor MIDI Settings.png" alt="Editor MIDI Settings" style="zoom: 80%;" />

> [!WARNING]
>
> *PitchGrid-Continuum Bridge must not be connected directly to the instrument.  Doing that causes a MIDI loop, which is indicated on the instrument's display.*
> <img src="images\Loop.png" alt="Loop" style="zoom: 100%;" />

#### Load Order

The order in which you load or turn on the connected components does not matter.  PitchGrid-Continuum Bridge will show messages advising you of anything that is not yet connected.
<img src="images\Awaiting PitchGrid Connection.png" alt="Awaiting PitchGrid Connection" style="zoom: 80%;" />
