# Attack Shark R1 Linux Driver (Rust)

Driver e utilitário CLI de alto desempenho para o mouse **Attack Shark R1** (dongle sem fio 2.4G e cabo USB-C cabeado) escrito **100% em Rust**, modelado com especificações de hardware **DDSL** através do toolkit [**device-driver**](https://device-driver.com/) e comunicação USB via [`rusb`](https://crates.io/crates/rusb).

---

## ⚡ Recursos

- **Consulta de Bateria:**
  - Compatibilidade direta com `-query-charge` (imprime o valor numérico exato, ex: `90`).
  - `--battery` / `-b`: porcentagem de carga da bateria.
  - `--json`: saída estruturada para integração com barras de status (**Waybar**, **Polybar**, **i3blocks**).
  - `--status` / `-s`: relatório legível com detalhes de conexão e bateria.
- **Configuração de Polling Rate:** 125Hz, 250Hz, 500Hz e 1000Hz.
- **Configuração de DPI:**
  - 6 estágios totalmente configuráveis entre 100 e 18000 DPI (em passos de 100).
  - Seleção de estágio ativo (1 a 6).
  - Tabela pré-computada com 180 códigos exatos de hardware.
- **Gerenciamento de Energia e Latência:**
  - `sleep-time`: tempo para sleep em segundos (0.5s a 30.0s).
  - `deep-sleep-time`: tempo para deep sleep em minutos (1m a 60m).
  - `key-response-time`: tempo de resposta/debounce em milissegundos (4ms a 50ms, pares).
- **Recursos do Sensor:**
  - `angle-snap`: ajuste angular (true/false).
  - `ripple-control`: controle de ruído de rastreamento (true/false).
- **Arquivo de Configuração INI:** Suporte a arquivos de configuração (`~/.config/attack-shark-r1.ini` ou `/etc/attack-shark-r1.ini`).
- **Seguro e Não-Intrusivo:** Utiliza apenas a interface USB vendor-specific (`Interface 2`), sem interromper os endpoints normais do cursor do mouse (`Interface 0`).

---

## 🛠️ Arquitetura e Especificação DDSL

O driver utiliza o framework [**device-driver**](https://device-driver.com/) conforme o livro [device-driver book](https://device-driver.com/book/). Os registros e relatórios do hardware USB são formalmente especificados em `attack_shark_r1.ddsl`:

```ddsl
device AttackSharkR1Device {
    register-address-type: u8,
    default-access: RW,
    default-byte-order: LE,

    register BatteryReport {
        address: 0x03,
        access: RO,
        fields: fieldset _ {
            size-bytes: 5,
            field report_id 7:0 -> uint,
            field header 15:8 -> uint,
            field status 23:16 -> uint,
            field flags 31:24 -> uint,
            field charge_raw 39:32 -> uint,
        },
    },

    register PollingRateConfig {
        address: 0x06,
        access: WO,
        ...
    },
    ...
}
```

Essa especificação é compilada pelo proc-macro `device_driver::compile!`, gerando structs de acesso a bits com verificação estática de tipos em Rust.

---

## 🚀 Instalação e Compilação

### Requisitos
- Rust 1.75+ (Cargo)
- `libusb-1.0` (presente na maioria das distribuições Linux)

### Compilação
```bash
cd ~/projects/attack-shark-r1
cargo build --release
```

O binário será gerado em `target/release/attack-shark-r1`.

### Instalação no Sistema
```bash
sudo make install
```
Isso instala o executável em `/usr/local/bin/attack-shark-r1` e adiciona as regras de udev em `/etc/udev/rules.d/99-attack-shark-r1.rules` para que qualquer usuário possa acessar o mouse sem `sudo`.

---

## 📖 Exemplos de Uso

### 1. Obter a porcentagem da bateria
```bash
# Compatibilidade com o driver antigo:
attack-shark-r1 -query-charge
# 90

# Ou usando a flag moderna:
attack-shark-r1 --battery
# 90
```

### 2. Status completo
```bash
attack-shark-r1 --status
```
Saída:
```text
=== Attack Shark R1 Status ===
Connection: Wireless (2.4G Receiver)
Battery:    90%
Raw Charge: 9/10
Status byte:0x40
```

### 3. Saída JSON (para Waybar / Polybar)
```bash
attack-shark-r1 --json
```
Saída:
```json
{"battery":90,"class":"normal","is_wired":false,"percentage":90,"raw_charge":9,"status_code":64,"tooltip":"Attack Shark R1: 90%"}
```

#### Exemplo de Módulo no Waybar (`~/.config/waybar/config`):
```json
"custom/mouse-battery": {
    "format": "󰍽 {}%",
    "interval": 60,
    "exec": "attack-shark-r1 --json",
    "return-type": "json",
    "tooltip": true
}
```

### 4. Ajustar Polling Rate
```bash
attack-shark-r1 -p 1000
```

### 5. Ajustar Estágio e Valores de DPI
```bash
# Definir estágio 2 ativo:
attack-shark-r1 --active-dpi 2

# Atualizar o valor do estágio 1 para 800 e estágio 2 para 1600:
attack-shark-r1 --dpi 1=800 --dpi 2=1600
```

### 6. Aplicar arquivo de configuração
```bash
# Gerar template de configuração:
attack-shark-r1 --generate-config > ~/.config/attack-shark-r1.ini

# Aplicar configurações:
attack-shark-r1 --reapply-config
```

---

## 📄 Regras do udev

Para usar o dispositivo sem permissão de superusuário (`root`), o arquivo `99-attack-shark-r1.rules` contém:
```udev
SUBSYSTEM=="usb", ATTR{idVendor}=="1d57", ATTR{idProduct}=="fa60", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="1d57", ATTR{idProduct}=="fa61", MODE="0666"
```

---

## 📦 Uso como Biblioteca Rust (`lib.rs`)

Você também pode utilizar o driver como dependência em outros projetos Rust:

```rust
use attack_shark_r1::{AttackSharkR1, PollingRate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mouse = AttackSharkR1::open()?;
    
    // Ler bateria
    let battery = mouse.get_battery_percentage()?;
    println!("Bateria: {battery}%");

    // Alterar polling rate para 1000Hz
    mouse.set_polling_rate(PollingRate::Hz1000)?;

    Ok(())
}
```

---

## ⚖️ Licença

MIT OR Apache-2.0.
