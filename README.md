#  mremote
> BLE-based light electric vehicle control system

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote) [![Dependabot Status](https://api.dependabot.com/badges/status?host=github&repo=chocol4te/mremote)](https://dependabot.com)

TODO

## Aims
* No connection issues *ever*
* Telemetry and configuration? of VESC from remote
* 100 hours per charge
* Comfortable - long journeys should not tire the user's hand

## Components
* nRF52810 microcontrollers
* SSD1306 display
* NCR18650GA battery
* Hall-effect thumb control

## TODO
### Software
- [ ] Basic radio functionality
- [ ] Interface with VESC
- [ ] Use ADC to read hall sensor
- [ ] Encrypt and authenticate packets
- [ ] Create telemetry display
- [ ] Hardware interrupts instead of polling
- [ ] Configuration menu for VESC
- [ ] VESC firmware update from remote

### Hardware
- [ ] Finalise breadboard design of both remote and receiver
- [ ] Case design
- [ ] PCB design of both remote and receiver


## Contributing

Issues and PRs very welcome :)

## License
Mozilla Public License Version 2.0([LICENSE](LICENSE) or
  https://www.mozilla.org/en-US/MPL/2.0/)
