RUSTC ?= rustc
RUSTFLAGS ?=

TEST_BINARY ?= ./run-tests
CLI_BINARY ?= ./linearscan

SRC ?=
SRC += src/linearscan.rs
SRC += src/linearscan/allocator.rs
SRC += src/linearscan/api.rs
SRC += src/linearscan/flatten.rs
SRC += src/linearscan/gap.rs
SRC += src/linearscan/generator.rs
SRC += src/linearscan/graph.rs
SRC += src/linearscan/json.rs
SRC += src/linearscan/liveness.rs

CLI_SRC ?=
CLI_SRC += bin/cli.rs

TEST_SRC ?=
TEST_SRC += test/runner.rs
TEST_SRC += test/emulator.rs

all: $(TEST_BINARY) $(CLI_BINARY)
	$(TEST_BINARY)

cli: $(CLI_BINARY)

clean:
	rm -f $(TEST_BINARY)

$(CLI_BINARY): $(SRC) $(CLI_SRC)
	$(RUSTC) $(RUSTFLAGS) bin/cli.rs -o $@

$(TEST_BINARY): $(SRC) $(TEST_SRC)
	$(RUSTC) $(RUSTFLAGS) --test test/runner.rs -o $@


.PHONY: all clean cli
