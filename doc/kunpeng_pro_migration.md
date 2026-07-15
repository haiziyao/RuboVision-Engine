# Kunpeng Pro Migration Requirements

## Goal

- Keep Raspberry Pi and Orange Pi Kunpeng Pro support in one branch.
- Build one Linux hardware implementation and switch board-specific device mapping through configuration.
- Keep RuboEngine, Function, Binding, Web, Dispatcher, Executor, and vision algorithms unchanged.
- Do not detect or install dependencies from runtime configuration.

## Common Hardware Implementation

- Replace `rppal::uart` with `serialport`.
- Preserve the current single UART owner thread, blocking reads, 100 ms read timeout, bounded output queue, and 500 ms reconnect delay.
- Replace `rppal::gpio` with the Linux GPIO character-device API through `gpiod`.
- Keep the current GPIO behavior: all configured status LEDs turn on when the first Function starts and turn off after the last concurrent Function finishes.
- Keep Camera access through OpenCV and V4L2.

## Board Configuration

Configuration selects the following values for each board:

- UART device path.
- GPIO chip and line mapping.
- GPIO active level.
- Camera device path.

- Raspberry Pi configuration is isolated in `config/raspberrypi`.
- Orange Pi configuration is isolated in `config/orangepi`.
- `config/application.yaml` keeps `config_path: config` and selects the board through `profile`.
- GPIO `chip` selects `/dev/gpiochipN`.
- Existing `run_pin` and `signals` values are line offsets inside that chip.
- Only `config/orangepi` and `config/raspberrypi` are accepted board configuration paths.
- Web listens on `0.0.0.0:3888` for LAN access.

The current Kunpeng Pro GPIO mapping is:

- `chip = 7`, `run_pin = 3`: GPIO7_03, physical pin 12.
- `chip = 7`, `color = 4`: GPIO7_04, physical pin 35.
- `chip = 7`, `qr = 5`: GPIO7_05, physical pin 40.

## Kunpeng Pro Environment Verified On 2026-07-14

- Device-tree model: `Orange Pi Ai Pro`.
- System: openEuler 22.03 LTS SP4, Linux 5.10, aarch64.
- Application UART: `/dev/ttyAMA1` or `/dev/ttyAMA2`; `/dev/ttyAMA0` is the debug UART.
- The `openEuler` user belongs to the `dialout` group and can access UART1 and UART2.
- GPIO character devices: `/dev/gpiochip0` through `/dev/gpiochip7`.
- GPIO devices are currently owned by `root:root` with mode `0600`; deployment must grant controlled access before running RuboVision as a non-root user.
- The board provides `/usr/bin/gpio_operate`, but it is executable only by root and is not the selected application backend.
- OpenCV 4.5.2 headers, libraries, pkg-config metadata, clang, CMake, GCC, and Rust 1.94 are available.
- A native OpenCV C++ compile and link check passed.
- No `/dev/video*` device existed during inspection; the Camera path must be confirmed after connecting the camera.

Before running as the `openEuler` user, grant controlled GPIO access:

```bash
sudo groupadd -f gpio
sudo usermod -aG gpio openEuler
echo 'SUBSYSTEM=="gpio", KERNEL=="gpiochip*", GROUP="gpio", MODE="0660"' | sudo tee /etc/udev/rules.d/99-rubo-gpio.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=gpio
```

Log out and reconnect after changing group membership.

## Raspberry Pi Compatibility

- UART uses the Raspberry Pi serial device path supplied by configuration.
- GPIO uses its `/dev/gpiochipN` device and line offsets supplied by configuration.
- Raspberry Pi-specific PWM, SPI, and other `rppal` functionality are outside the current RuboVision requirements.

## Required Code Changes

- The `hardware` Cargo feature enables the Engine implementations backed by `serialport` and `gpiod`.
- Engine provides the shared `UartSource`, `UartSink`, and `GpioSink`; RuboVision only supplies profile-specific configuration and business behavior.
- File configuration and code-generated defaults contain matching board values.
- Hardware test descriptions refer to Linux board requirements.
- Do not copy a Windows `target` directory; build the release binary on aarch64.

## Validation

- Build without hardware and OpenCV features.
- Build with `opencv,hardware` on Kunpeng Pro.
- Verify UART input creates one Message per received command byte.
- Verify UART output sends only the Function result value followed by a newline.
- Verify UART disconnect does not cause a busy loop.
- Verify concurrent Functions keep LEDs active until the last Function completes.
- Verify both board configurations load without changing application code.
- Verify Camera and the four manually launched Function tests after `/dev/videoN` is available.
