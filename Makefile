all: install

clean:
	rm -r target

install:
	cargo build
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/debug/mremote target/thumbv7m-none-eabi/debug/mremote.bin
	st-flash --reset write target/thumbv7m-none-eabi/debug/mremote.bin 0x08000000
