
// constants

pub const CMD_STOP:u8 = 0;
pub const CMD_RETURN_KV:u8 = 1;
pub const CMD_SET_KV:u8 = 2;

pub const VBIN_NOTHING:u8 = 0;
pub const VBIN_BOOL:u8 = 1;
pub const VBIN_ABOOL:u8 = 2;
pub const VBIN_CMAP_BEGIN:u8 = 3;
pub const VBIN_CMAP_END:u8 = 4;

pub const CMAPB_KEY:u8 = 2;

pub const DBSTATE_START:u8 = 0;
pub const DBSTATE_OK:u8 = 1;
pub const DBSTATE_SHUTTING_DOWN:u8 = 2;