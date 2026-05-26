CC ?= gcc
CFLAGS ?= -Wall -Wextra -std=c11 -I../../include
RUST_TARGET_DIR ?= ../../target
LIB := $(RUST_TARGET_DIR)/release/libco2_panel_c_api.a

all: demo

$(LIB):
	cargo build --release -p co2_panel_c_api

demo: main.c $(LIB)
	$(CC) $(CFLAGS) main.c $(LIB) -ldl -lpthread -lm -o demo

run: demo
	./demo

clean:
	rm -f demo

.PHONY: all run clean

