PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
RULESDIR ?= /etc/udev/rules.d

all: build

build:
	cargo build --release

test:
	cargo test

install: build
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/attack-shark-r1 $(DESTDIR)$(BINDIR)/attack-shark-r1
	install -d $(DESTDIR)$(RULESDIR)
	install -m 644 99-attack-shark-r1.rules $(DESTDIR)$(RULESDIR)/99-attack-shark-r1.rules
	udevadm control --reload-rules || true
	udevadm trigger || true

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/attack-shark-r1
	rm -f $(DESTDIR)$(RULESDIR)/99-attack-shark-r1.rules
	udevadm control --reload-rules || true

clean:
	cargo clean

.PHONY: all build test install uninstall clean
