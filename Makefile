RUSTC ?= rustc
RUSTFLAGS ?=

TEST_BINARY ?= ./run-tests

SRC ?=
SRC += src/linearscan.rs
SRC += src/tests.rs

all: $(TEST_BINARY)
	$(TEST_BINARY)

clean:
	rm -f $(TEST_BINARY)

$(TEST_BINARY): $(SRC)
	$(RUSTC) $(RUSTFLAGS) --test src/tests.rs -o $@


.PHONY: all clean
