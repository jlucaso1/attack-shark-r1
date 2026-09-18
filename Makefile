PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
RULESDIR ?= /etc/udev/rules.d
SYSTEMDDIR ?= /etc/systemd/system

TARGET = target/release/attack-shark-r1

all: build

build: $(TARGET)

$(TARGET):
	cargo build --release

test:
	cargo test

install:
	@if [ ! -f $(TARGET) ]; then \
		echo "Error: $(TARGET) not found."; \
		echo "Please build the project first without sudo:"; \
		echo "  cargo build --release"; \
		exit 1; \
	fi
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 $(TARGET) $(DESTDIR)$(BINDIR)/attack-shark-r1
	install -d $(DESTDIR)$(RULESDIR)
	install -m 644 99-attack-shark-r1.rules $(DESTDIR)$(RULESDIR)/99-attack-shark-r1.rules
	install -d $(DESTDIR)$(SYSTEMDDIR)
	install -m 644 attack-shark-r1.service $(DESTDIR)$(SYSTEMDDIR)/attack-shark-r1.service
	udevadm control --reload-rules || true
	udevadm trigger || true
	systemctl daemon-reload || true

uninstall:
	systemctl stop attack-shark-r1 || true
	systemctl disable attack-shark-r1 || true
	rm -f $(DESTDIR)$(BINDIR)/attack-shark-r1
	rm -f $(DESTDIR)$(RULESDIR)/99-attack-shark-r1.rules
	rm -f $(DESTDIR)$(SYSTEMDDIR)/attack-shark-r1.service
	udevadm control --reload-rules || true
	systemctl daemon-reload || true

clean:
	cargo clean

.PHONY: all build test install uninstall clean
