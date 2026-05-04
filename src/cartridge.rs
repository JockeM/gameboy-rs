pub(crate) struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mapper: Mapper,
    rom_bank_count: usize,
    ram_bank_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CartridgeSnapshot {
    ram: Vec<u8>,
    mapper: Mapper,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mapper {
    NoMbc,
    Mbc1 {
        ram_enabled: bool,
        rom_bank_low5: u8,
        bank_high2: u8,
        banking_mode: u8,
    },
}

impl Cartridge {
    pub(crate) fn new(rom_bytes: &[u8]) -> Self {
        let rom = rom_bytes.to_vec();
        let rom_bank_count = rom.len().max(0x4000).div_ceil(0x4000);
        let ram_size = rom_bytes
            .get(0x149)
            .copied()
            .map(cartridge_ram_size)
            .unwrap_or(0);
        let ram_bank_count = ram_size.div_ceil(0x2000);
        let mapper = match rom_bytes.get(0x147).copied().unwrap_or(0x00) {
            0x00 => Mapper::NoMbc,
            0x01..=0x03 => Mapper::Mbc1 {
                ram_enabled: false,
                rom_bank_low5: 1,
                bank_high2: 0,
                banking_mode: 0,
            },
            cartridge_type => panic!("Unsupported cartridge type: {cartridge_type:#04X}"),
        };

        Self {
            rom,
            ram: vec![0; ram_size],
            mapper,
            rom_bank_count,
            ram_bank_count,
        }
    }

    pub(crate) fn rom_fingerprint(&self) -> u64 {
        self.rom.iter().fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        }) ^ self.rom.len() as u64
    }

    pub(crate) fn save_snapshot(&self) -> CartridgeSnapshot {
        CartridgeSnapshot {
            ram: self.ram.clone(),
            mapper: self.mapper.clone(),
        }
    }

    pub(crate) fn load_snapshot(&mut self, snapshot: &CartridgeSnapshot) {
        self.ram.clone_from(&snapshot.ram);
        self.mapper = snapshot.mapper.clone();
    }

    pub(crate) fn write_rom(&mut self, address: u16, value: u8) {
        match &mut self.mapper {
            Mapper::NoMbc => {}
            Mapper::Mbc1 {
                ram_enabled,
                rom_bank_low5,
                bank_high2,
                banking_mode,
            } => match address {
                0x0000..=0x1FFF => *ram_enabled = value & 0x0F == 0x0A,
                0x2000..=0x3FFF => {
                    let bank = value & 0x1F;
                    *rom_bank_low5 = if bank == 0 { 1 } else { bank };
                }
                0x4000..=0x5FFF => *bank_high2 = value & 0x03,
                0x6000..=0x7FFF => *banking_mode = value & 0x01,
                _ => unreachable!("invalid ROM write address {address:#06X}"),
            },
        }
    }

    pub(crate) fn read_ram(&self, address: u16) -> u8 {
        self.ram_index(address)
            .and_then(|index| self.ram.get(index).copied())
            .unwrap_or(0xFF)
    }

    pub(crate) fn write_ram(&mut self, address: u16, value: u8) {
        if let Some(index) = self.ram_index(address) {
            self.ram[index] = value;
        }
    }

    pub(crate) fn is_present(&self) -> bool {
        !self.rom.is_empty()
    }

    pub(crate) fn sync_visible_memory(&self, mem: &mut [u8; 0x10000]) {
        self.copy_rom_bank_into(mem, 0x0000, self.lower_rom_bank());
        self.copy_rom_bank_into(mem, 0x4000, self.upper_rom_bank());
        self.sync_external_ram(mem);
    }

    fn copy_rom_bank_into(&self, mem: &mut [u8; 0x10000], start: usize, bank: usize) {
        let window = &mut mem[start..start + 0x4000];
        window.fill(0xFF);

        let offset = bank * 0x4000;
        let available = self.rom.len().saturating_sub(offset).min(0x4000);
        if available > 0 {
            window[..available].copy_from_slice(&self.rom[offset..offset + available]);
        }
    }

    pub(crate) fn sync_external_ram(&self, mem: &mut [u8; 0x10000]) {
        let window = &mut mem[0xA000..0xC000];
        window.fill(0xFF);

        if self.ram.is_empty() {
            return;
        }

        let bank = self.current_ram_bank();
        let offset = bank * 0x2000;
        let available = self.ram.len().saturating_sub(offset).min(0x2000);
        if available > 0 {
            window[..available].copy_from_slice(&self.ram[offset..offset + available]);
        }
    }

    fn lower_rom_bank(&self) -> usize {
        match self.mapper {
            Mapper::NoMbc => 0,
            Mapper::Mbc1 {
                bank_high2,
                banking_mode,
                ..
            } if banking_mode != 0 => (usize::from(bank_high2) << 5) % self.rom_bank_count,
            Mapper::Mbc1 { .. } => 0,
        }
    }

    fn upper_rom_bank(&self) -> usize {
        match self.mapper {
            Mapper::NoMbc => {
                if self.rom_bank_count > 1 {
                    1
                } else {
                    0
                }
            }
            Mapper::Mbc1 {
                rom_bank_low5,
                bank_high2,
                ..
            } => {
                let bank = ((usize::from(bank_high2) << 5) | usize::from(rom_bank_low5))
                    % self.rom_bank_count;
                if bank == 0 && self.rom_bank_count > 1 {
                    1
                } else {
                    bank
                }
            }
        }
    }

    fn current_ram_bank(&self) -> usize {
        if self.ram_bank_count == 0 {
            return 0;
        }

        match self.mapper {
            Mapper::Mbc1 {
                bank_high2,
                banking_mode,
                ..
            } if banking_mode != 0 => usize::from(bank_high2) % self.ram_bank_count,
            _ => 0,
        }
    }

    fn ram_index(&self, address: u16) -> Option<usize> {
        if self.ram.is_empty() || !(0xA000..=0xBFFF).contains(&address) {
            return None;
        }

        match self.mapper {
            Mapper::Mbc1 { ram_enabled, .. } if !ram_enabled => None,
            _ => {
                let offset = usize::from(address - 0xA000);
                let index = self.current_ram_bank() * 0x2000 + offset;
                (index < self.ram.len()).then_some(index)
            }
        }
    }
}

fn cartridge_ram_size(code: u8) -> usize {
    match code {
        0x00 => 0,
        0x01 => 0x0800,
        0x02 => 0x2000,
        0x03 => 0x8000,
        0x04 => 0x20000,
        0x05 => 0x10000,
        _ => 0,
    }
}
