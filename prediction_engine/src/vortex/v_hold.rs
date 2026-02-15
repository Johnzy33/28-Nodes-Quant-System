
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FullSessionSpec {
    pub is_active: i32,
    pub open_hour: i32,
    pub open_min: i32,
    pub close_hour: i32,
    pub close_min: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AssetInfoPacket {
    pub asset: [u8; 16],
    // --- Integer Properties ---
    pub digits: i32,
    pub stops_level: i32,
    pub trade_mode: i32,
    pub calc_mode: i32,
    pub execution_mode: i32,
    pub gtc_mode: i32,
    pub swap_mode: i32,
    pub swap_rollover3days: i32, // Sunday=0, Monday=1...
    
    // --- Double Properties ---
    pub contract_size: f64,
    pub tick_size: f64,
    pub tick_value: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub swap_long: f64,
    pub swap_short: f64,
    pub margin_initial: f64,     // From your screenshot: Margin rates initial
    pub margin_maintenance: f64, // From your screenshot: Margin rates maintenance
    
    pub day_of_week: i32, // 0 = Sunday...6 = Saturday
    pub is_active: i32,   // 1 if trading is possible today
    pub open_hour: i32,
    pub open_min: i32,
    pub close_hour: i32,
    pub close_min: i32,
}


#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FullSessionSpec {
    pub day_of_week: i32, // 0 = Sunday...6 = Saturday
    pub is_active: i32,   // 1 if trading is possible today
    pub open_hour: i32,
    pub open_min: i32,
    pub close_hour: i32,
    pub close_min: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AssetInfoPacket {
    pub asset: [u8; 16],
    // Integer properties
    pub digits: i32,
    pub trade_mode: i32,
    pub calc_mode: i32,
    pub execution_mode: i32,
    pub swap_mode: i32,
    pub swap_rollover3days: i32,
    pub _pad: i32, // Padding for 8-byte alignment of doubles

    // Double properties
    pub contract_size: f64,
    pub tick_size: f64,
    pub tick_value: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub swap_long: f64,
    pub swap_short: f64,
    pub margin_initial: f64,

    // The 7-Day Session Table (Contiguous in memory)
    pub sessions: [FullSessionSpec; 7],
}