//+------------------------------------------------------------------+
//|                                                ice_pipe_data.mq5 |
//|                                  Copyright 2026, MetaQuotes Ltd. |
//|                                             https://www.mql5.com |
//+------------------------------------------------------------------+
#property copyright "Copyright 2026, MetaQuotes Ltd."
#property link      "https://www.mql5.com"
#property version   "1.00"
// MQL5 side (Put in your EA's global scope)
// Tell MT5 where to find your functions
#import "mt5_bridge.dll"
   bool InitIceoryx();
   void SendTick(uint id, double bid, double ask, ulong vol);
#import

// Global variable to track if Iceoryx is ready
bool IceoryxInitialized = false;

int OnInit() {
    // Call your Rust function!
    IceoryxInitialized = InitIceoryx();
    
    if(!IceoryxInitialized) {
        Print("Rust DLL: Failed to initialize Iceoryx2. Check if /dev/shm is full.");
        return(INIT_FAILED);
    }
    
    Print("Rust DLL: Iceoryx2 initialized successfully.");
    return(INIT_SUCCEEDED);
}


void OnTick() {
   // Send current symbol data to the Rust bus
   SendTick(1, SymbolInfoDouble(_Symbol, SYMBOL_BID), 
               SymbolInfoDouble(_Symbol, SYMBOL_ASK), 
               (ulong)SymbolInfoInteger(_Symbol, SYMBOL_VOLUME));
}