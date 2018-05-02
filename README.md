#  mremote
> Encrypted electric longboard remote with telemetry and lighting

## Building
[![Build Status](https://travis-ci.org/chocol4te/mremote.svg?branch=master)](https://travis-ci.org/chocol4te/mremote)

```cargo build --release``` builds an ELF file at ```./target/thumbv7m-none-eabi/release/mremote```.

```make``` builds the project, creates a binary and flashes it using st-flash.


## Features
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
- [x] Implement RTFM
- [ ] Configuartion menu for VESC
- [ ] VESC firmware update from remote
- [ ] Customisation of screen

### Hardware
- [ ] Finalise breadboard design of both remote and receiver
- [ ] PCB design of both remote and receiver
- [ ] Design enclosure, borrowing heavily from [solidgeek's NRF remote](http://www.electric-skateboard.builders/t/simple-3d-printed-nrf-remote-arduino-controlled/28543)


## License
Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
