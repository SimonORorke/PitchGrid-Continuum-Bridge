# PitchGrid-Continuum Bridge

PitchGrid-Continuum Bridge is a bridging application that allows tunings specified in [PitchGrid](https://pitchgrid.io/) to tune a [Haken Continuum fingerboard](https://www.hakenaudio.com/).

<img src="images\PitchGrid-Continuum Bridge.png" alt="PitchGrid-Continuum Bridge" style="zoom: 80%;" />

### Supported Instruments

PitchGrid-Continuum Bridge (PCB) has so far only been tested with the Continuum.  But it should also work with the ContinuuMini, EaganMatrix Eurorack Module and EaganMatrix Micro. For the latter two instruments, connection to an MPE or MIDI keyboard would enable best use of PitchGrid and PCB, though other configurations may be possible.  The Osmose is not supported:  although it has the EaganMatrix sound engine in common with those other instruments, it does not provide a public API that would support tuning.

### Supported Haken Audio Firmware Versions

Custom tuning is broken in Haken Audio Firmware v10.52, the latest production version.  It is fixed in the subsequent beta versions, the most recent of which is available for anyone to download.
