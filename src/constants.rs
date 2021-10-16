
// constants

//cmd types
pub const CMD_STOP:u8 = 0;
pub const CMD_RETURN_KV:u8 = 1;
pub const CMD_SET_KV:u8 = 2;
pub const CMD_OP_ATOMIC:u8 = 3;
pub const CMD_OP_NORMAL:u8 = 4;

//data types
pub const VBIN_NOTHING:u8 = 0;
pub const VBIN_BOOL:u8 = 1;
pub const VBIN_ABOOL:u8 = 2;
pub const VBIN_CMAP_BEGIN:u8 = 3;
pub const VBIN_CMAP_END:u8 = 4;
pub const VBIN_ERROR:u8 = 5;
pub const VBIN_UINT:u8 = 6;
pub const VBIN_AUINT:u8 = 7;
pub const VBIN_IINT:u8 = 8;
pub const VBIN_AIINT:u8 = 9;

//symbolizes intra-map key
pub const CMAPB_KEY:u8 = 2;

//atomic ops
// These are a bit moe numerous so better to do u16
pub const OP_ATOMIC_STORE:u16 = 0;
pub const OP_ATOMIC_STORE_RELAX:u16 = 1;
pub const OP_ATOMIC_SWAP:u16 = 2;
pub const OP_ATOMIC_SWAP_RELAX:u16 = 3;
pub const OP_ATOMIC_COND_STORE:u16 = 4;
pub const OP_ATOMIC_COND_STORE_RELAX:u16 = 5;
pub const OP_ATOMIC_COND_SWAP:u16 = 6;
pub const OP_ATOMIC_COND_SWAP_RELAX:u16 = 7;
pub const OP_ATOMIC_ADD:u16 = 8;
pub const OP_ATOMIC_ADD_FETCH:u16 = 9;
pub const OP_ATOMIC_SUB:u16 = 10;
pub const OP_ATOMIC_SUB_FETCH:u16 = 11;

// normal ops
pub const OP_NORM_UPDATE:u16 = 0;

//errors
pub const ERR_DATE_TIME:u8 = 0;
pub const ERR_RET_NOT_FOUND:u8 = 1;
pub const ERR_UNEXPECT_BYTE:u8 = 2;
pub const ERR_TYPE_NOT_ATOMIC:u8 = 3;
pub const ERR_OPER_NOT_SUPPORTED:u8 = 4; // operation isn't supported for type

//db states
pub const DBSTATE_START:u8 = 0;
pub const DBSTATE_OK:u8 = 1;
pub const DBSTATE_SHUTTING_DOWN:u8 = 2;

pub const SSEQ_U8_EQ:&'static [u8] = "=".as_bytes();