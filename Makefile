all: install

clean:
	rm -r target/

install:
	xargo build --release
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/release/mremote target/thumbv7m-none-eabi/release/mremote.bin
	st-flash --reset write target/thumbv7m-none-eabi/release/mremote.bin 0x08000000
