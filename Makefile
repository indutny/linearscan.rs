RUSTC ?= rustc
RUSTFLAGS ?=

TEST_BINARY ?= ./run-tests

SRC ?=
SRC += src/linearscan.rs
SRC += src/linearscan/allocator.rs
SRC += src/linearscan/dce.rs
SRC += src/linearscan/flatten.rs
SRC += src/linearscan/gap.rs
SRC += src/linearscan/generator.rs
SRC += src/linearscan/graph.rs
SRC += src/linearscan/json.rs
SRC += src/linearscan/liveness.rs
SRC += test/runner.rs
SRC += test/emulator.rs

all: $(TEST_BINARY)
	$(TEST_BINARY)

clean:
	rm -f $(TEST_BINARY)

$(TEST_BINARY): $(SRC)
	$(RUSTC) $(RUSTFLAGS) --test test/runner.rs -o $@


.PHONY: all clean
