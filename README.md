#  mremote
> Encrypted electric longboard remote with telemetry

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote) [![Dependabot Status](https://api.dependabot.com/badges/status?host=github&repo=chocol4te/mremote)](https://dependabot.com)

TODO

## Aims
* No connection issues *ever*
* Telemetry and configuration? of VESC from remote
* 100 hours per charge
* Comfortable - long journeys should not tire the user's hand

## Components
* STM32F103C8 microcontrollers
* SSD1306 display
* CC1101 radios (NRF24L01 temporarily)
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

Issues and PRs very welcome, nothing is too small.

## License
GNU Affero General Public License v3.0([LICENSE](LICENSE) or
  https://www.gnu.org/licenses/agpl-3.0.txt)
