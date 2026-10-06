use super::*;
//
// pub enum ARM9_DISPLAY_ENGINE_A {
// 	DISPCNT_ENGINE_A //0x4000000 // 4    2D Engine A - DISPCNT - LCD Control (Read/Write)
// 	DISPSTAT_ENGINE_A_B // 0x4000004 // 2    2D Engine A+B - DISPSTAT - General LCD Status (Read/Write)
// 	VCOUNT_ENGINE_A_B // 0x4000006 // 2    2D Engine A+B - VCOUNT - Vertical Counter (Read only)
// 	2D_Engine_A //0x4000008 // 50h  2D Engine A (same registers as GBA, some changed bits)
// 	DISP3DCNT //0x4000060 // 2    DISP3DCNT - 3D Display Control Register (R/W)
// 	DISPCAPCNT// 0x4000064 // 4    DISPCAPCNT - Display Capture Control Register (R/W)
// 	DISP_MMEM_FIFO// 0x4000068 // 4    DISP_MMEM_FIFO - Main Memory Display FIFO (R?/W)
// 	MASTER_BRIGHT//0x400006C // 2    2D Engine A - MASTER_BRIGHT - Master Brightness Up/Down
// }
//
// pub enum GBA_IO_MAP{
//
// }
//
// pub enum ARM9_DMA_TIMERS_AND_KEYPAD {
// DMA_CHANNEL ,// 0x40000B0 // 30h  DMA Channel 0..3
// DMA_FILL_REGISTER ,// 0x40000E0 // 10h  DMA FILL Registers for Channel 0..3
// Timers, // 0x4000100 // 10h  Timers 0..3
// KEYINPUT, // 0x4000130 // 2    KEYINPUT
// KEYCNT, //0x4000132 // 2    KEYCNT
// }
//
// pub enum ARM9_IPC_SLASH_ROM {
// IPCSYNC, // 0x4000180 // 2  IPCSYNC - IPC Synchronize Register (R/W)
// IPCFIFOCNT, // 0x4000184 // 2  IPCFIFOCNT - IPC Fifo Control Register (R/W)
// IPCFIFOSEND, // 0x4000188 // 4  IPCFIFOSEND - IPC Send Fifo (W)
// AUXSPICNT, // 0x40001A0 // 2  AUXSPICNT - Gamecard ROM and SPI Control
// AUXSPIDATA, // 0x40001A2 // 2  AUXSPIDATA - Gamecard SPI Bus Data/Strobe
// GAMECARD, // 0x40001A4 // 4  Gamecard bus timing/control
// 0x40001A8 // 8  Gamecard bus 8-byte command out
//0x40001B0 // 4  Gamecard Encryption Seed 0 Lower 32bit
// 0x40001B4 // 4  Gamecard Encryption Seed 1 Lower 32bit
// 0x40001B8 // 2  Gamecard Encryption Seed 0 Upper 7bit (bit7-15 unused)
// 0x40001BA // 2  Gamecard Encryption Seed 1 Upper 7bit (bit7-15 unused)

// 	ARM9_MEMORY_AND_IRQ_CONTROL
// 0x4000204 // 2  EXMEMCNT - External Memory Control (R/W)
// 0x4000208 // 2  IME - Interrupt Master Enable (R/W)
// 0x4000210 // 4  IE  - Interrupt Enable (R/W)
// 0x4000214 // 4  IF  - Interrupt Request Flags (R/W)
// 0x4000240 // 1  VRAMCNT_A - VRAM-A (128K) Bank Control (W)
// 0x4000241 // 1  VRAMCNT_B - VRAM-B (128K) Bank Control (W)
// 0x4000242 // 1  VRAMCNT_C - VRAM-C (128K) Bank Control (W)
// 0x4000243 // 1  VRAMCNT_D - VRAM-D (128K) Bank Control (W)
// 0x4000244 // 1  VRAMCNT_E - VRAM-E (64K) Bank Control (W)
// 0x4000245 // 1  VRAMCNT_F - VRAM-F (16K) Bank Control (W)
// 0x4000246 // 1  VRAMCNT_G - VRAM-G (16K) Bank Control (W)
// 0x4000247 // 1  WRAMCNT   - WRAM Bank Control (W)
// 0x4000248 // 1  VRAMCNT_H - VRAM-H (32K) Bank Control (W)
// 0x4000249 // 1  VRAMCNT_I - VRAM-I (16K) Bank Control (W)
/*
ARM9_DISPLAY_ENGINE_B,
4001000h  4    2D Engine B - DISPCNT - LCD Control (Read/Write)
4001008h  50h  2D Engine B (same registers as GBA, some changed bits)
400106Ch  2    2D Engine B - MASTER_BRIGHT - 16bit - Brightness Up/Down*/

pub enum ARM9_IO_MAPS {
    ARM9_DISPLAY_ENGINE_A,
    ARM9_DMA_TIMERS_AND_KEYPAD,
    ARM9_IPC_SLASH_ROM,
    ARM9_MEMORY_AND_IRQ_CONTROL,
    ARM9_MATHS,
    ARM9_3D_DISPLAY_ENGINE,
    ARM9_DISPLAY_ENGINE_B,
    ARM9_DSi_Extra_Registers,
    ARM9_DS_Debug_Registers_For_Emulator_Slash_Devkits,
    ARM9_Hardcoded_RAM_Addresses_for_Exception_Handling,
    Main_Memory_Control,
}
#[derive(Debug, Clone, Copy)]
pub struct TryFromAsmWordToArm9IoMapsError;

impl TryFrom<AsmWord> for ARM9_IO_MAPS {
    type Error = TryFromAsmWordToArm9IoMapsError;

    fn try_from(value: AsmWord) -> Result<Self, Self::Error> {
        if let Some(addr) = value.get_address() {
            let Arm9IoMaps = match addr {
                0x27FFFFE => ARM9_IO_MAPS::Main_Memory_Control,
                0x4000000..=0x400006C => ARM9_IO_MAPS::ARM9_DISPLAY_ENGINE_A,
                0x40000B0..=0x4000132 => ARM9_IO_MAPS::ARM9_DMA_TIMERS_AND_KEYPAD,
                0x4000180..=0x40001BA => ARM9_IO_MAPS::ARM9_IPC_SLASH_ROM,
                0x4000204..=0x4000249 => ARM9_IO_MAPS::ARM9_MEMORY_AND_IRQ_CONTROL,
                0x4000280..0x4000304 => ARM9_IO_MAPS::ARM9_MATHS,
                0x4000320..0x40006A3 => ARM9_IO_MAPS::ARM9_3D_DISPLAY_ENGINE,
                0x4001000..0x400106C => ARM9_IO_MAPS::ARM9_DISPLAY_ENGINE_B,

                _ => todo!(),
            };
            return Ok(Arm9IoMaps);
        } else {
            Err(TryFromAsmWordToArm9IoMapsError)
        }
    }
}
