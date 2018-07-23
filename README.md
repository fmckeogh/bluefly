#  mremote
> Encrypted electric longboard remote with telemetry and lighting

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote)

```make tx``` builds and flashes the firmware for the controller
```make rx``` builds and flashes the firmware for the receiver

## Aims
* No connection issues *ever*
* Full telemetry and configuration of VESC from remote
* 100 hours per charge
* Comfortable - long journeys should not tire the user's hand

## Components
* STM32F103C8
* SSD1306 display
* CC1101 radios
* NCR18650GA
* 50mm SoftPot

## TODO
### Software
- [ ] Create telemetry display
- [x] Get radios working (packet abstraction?)
- [ ] ~Implement RTFM~
- [ ] Configuration menu for VESC
- [ ] VESC firmware update from remote
- [ ] Customization of screen

### Hardware
- [x] Finalise breadboard design of both remote and receiver
- [ ] Case design
- [ ] PCB design of both remote and receiver


## Contributing

Issues and PRs very welcome, nothing is too small.

All PRs must pass all checks, have run `cargo fmt` and `cargo fix`.

## License
GNU Affero General Public License v3.0([LICENSE](LICENSE) or
  https://www.gnu.org/licenses/agpl-3.0.txt)
