#!/bin/bash
stty -F /dev/serial/by-id/usb-Fake_company_Serial_port_TEST-if00 1200 || true
PI_DIRECTORY="/run/media/$USER/RPI-RP2"
while [ ! -d "$PI_DIRECTORY" ]; do
    sleep 0.5
done

elf2uf2-rs -d $@
