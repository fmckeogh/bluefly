#  μremote
> Encrypted electric longboard remote with telemetry and lighting

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote)

```cargo build --release``` builds an ELF file at ```./target/thumbv7m-none-eabi/release/mremote```.

```make``` builds the project, creates a binary and flashes it using st-flash.

## Aims
* No connection issues ever
* Full telemetry and configuration of VESC from remote
* 100 hours per charge (acheiveable with single LG MJ1, final design aims for 2)
* Comfortable - long journeys should not tire the user's hand

## Components
* STM32L081KZ (subject to change)
* SSD1306 display (looking for something larger OR that draws less current)
* CC1101 radio
* 2x LG MJ1 batteries
* SS49E hall effect sensor (lying around on my desk)

## TODO
### Software 
- [ ] Create telemetry display
- [ ] Get radios working (packet abstraction?)
- [ ] Implement RTFM
- [ ] Configuartion menu for VESC
- [ ] VESC firmware update from remote
- [ ] Customisation of screen

### Hardware
- [ ] Finalise breadboard design of both remote and receiver
- [ ] PCB design of both remote and receiver
- [ ] Design enclosure, thumb wheel, maybe forefinger hole?


## Contributing

Issues and PRs very welcome, nothing is too small.

All PRs must pass all checks, have run `cargo fmt` and `cargo clippy`.

## License
GNU Affero General Public License v3.0([LICENSE](LICENSE) or
  https://www.gnu.org/licenses/agpl-3.0.txt)
