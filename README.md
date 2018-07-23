#  mremote
> Encrypted electric longboard remote with telemetry

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote)

```make tx``` builds and flashes the firmware for the controller

```make rx``` builds and flashes the firmware for the receiver

## Aims
* No connection issues *ever*
* Telemetry and configuration of VESC from remote
* 100 hours per charge
* Comfortable - long journeys should not tire the user's hand

## Components
* STM32F103C8 microcontrollers
* SSD1306 display
* CC1101 radios
* NCR18650GA battery
* 50mm SoftPot

## TODO
### Software
- [x] Basic radio functionality
- [x] Interface with VESC
- [x] Use ADC to read SoftPot
- [ ] Encrypt and authenticate packets
- [ ] Create telemetry display
- [ ] Hardware interrupts instead of polling
- [ ] RTFM?
- [ ] Configuration menu for VESC
- [ ] VESC firmware update from remote

### Hardware
- [ ] Finalise breadboard design of both remote and receiver
- [ ] Case design
- [ ] PCB design of both remote and receiver


## Contributing

Issues and PRs very welcome, nothing is too small.

All PRs must have run `cargo fmt` and `cargo fix`.

## License
GNU Affero General Public License v3.0([LICENSE](LICENSE) or
  https://www.gnu.org/licenses/agpl-3.0.txt)
