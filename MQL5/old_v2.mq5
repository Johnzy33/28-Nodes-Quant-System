//+------------------------------------------------------------------+
//|                                                pipe_data_v2.mq5 |
//|                                Copyright 2026, 28th Node Systems |
//+------------------------------------------------------------------+
#property copyright "Copyright 2026, 28th Node Systems"
#property link      "28thNodesystem.com"
#property version   "2.00"
#property strict

// --- Data Structures (8-byte aligned for Rust) ---

struct MultiplexedTick {
   char   asset_name[10];
   char   _dummy[6];      // Padding
   double bid;
   double ask;
   long   time_msc;
};

struct SyncBar {
   char   asset[10];
   char   _pad[6];
   double open;
   double high;
   double low;
   double close;
   long   volume;
   long   time;
};

struct SessionPacket {
   char   asset[16];
   uint   open_hour;
   uint   open_min;
   uint   close_hour;
   uint   close_min;
   uint   is_active;      // 1 = Session exists, 0 = No session today
};

struct SubscribedAsset {
   string symbol;
   ENUM_TIMEFRAMES timeframe;
   double last_sent_bid;
   double last_sent_ask;
};

// --- Global Variables ---
SubscribedAsset Subscriptions[];
int socket_handle = INVALID_HANDLE;
uint last_reconnect_attempt = 0;

// --- Communication Helpers ---

template<typename T>
void SafeSend(uchar header_type, T &struct_obj) {
   if(socket_handle == INVALID_HANDLE || !SocketIsConnected(socket_handle)) return;
   
   uchar header[1] = {header_type};
   uchar data[];
   StructToCharArray(struct_obj, data);
   
   SocketSend(socket_handle, header, 1);
   SocketSend(socket_handle, data, ArraySize(data));
}

bool EnsureConnection() {
   if(socket_handle != INVALID_HANDLE && SocketIsConnected(socket_handle)) return true;

   // 5-second backoff to prevent log spam and CPU spikes
   if(GetTickCount() - last_reconnect_attempt < 5000) return false;
   last_reconnect_attempt = GetTickCount();

   if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
   socket_handle = SocketCreate();

   if(SocketConnect(socket_handle, "127.0.0.1", 9090, 100)) {
      Print("📡 Connected to Rust Engine.");
      // Send initial calibration on connect
      SendCalibration();
      return true;
   }
   return false;
}

// --- Logic Modules ---

void SendCalibration() {
   long times[2];
   times[0] = TimeCurrent(); // Broker Time
   times[1] = TimeLocal();   // System Time
   SafeSend(254, times);
}

void SendSessionDefinitions(string symbol) {
   MqlDateTime dt;
   TimeCurrent(dt);
   datetime start, end;
   SessionPacket packet;
   ZeroMemory(packet);
   StringToCharArray(symbol, packet.asset, 0, 16);

   if(SymbolInfoSessionQuote(symbol, (ENUM_DAY_OF_WEEK)dt.day_of_week, 0, start, end)) {
      packet.open_hour = (uint)(start / 3600);
      packet.open_min  = (uint)((start % 3600) / 60);
      packet.close_hour = (uint)(end / 3600);
      packet.close_min  = (uint)((end % 3600) / 60);
      packet.is_active = 1;
   }
   SafeSend(3, packet);
}

void PollMarketData() {
   for(int i=0; i<ArraySize(Subscriptions); i++) {
      MqlTick tick;
      if(SymbolInfoTick(Subscriptions[i].symbol, tick)) {
         // Only send if price actually moved
         if(tick.bid == Subscriptions[i].last_sent_bid && tick.ask == Subscriptions[i].last_sent_ask) continue;

         MultiplexedTick p;
         ZeroMemory(p);
         StringToCharArray(Subscriptions[i].symbol, p.asset_name, 0, 10);
         p.bid = tick.bid;
         p.ask = tick.ask;
         p.time_msc = tick.time_msc;

         SafeSend(0, p); // Header 0: Live Ticks
         Subscriptions[i].last_sent_bid = tick.bid;
         Subscriptions[i].last_sent_ask = tick.ask;
      }
   }
}

// --- Command Listener (Rust -> MT5) ---

void CheckForIncomingCommands() {
   uint readable = SocketIsReadable(socket_handle);
   if(readable < 1) return;

   uchar head[1];
   SocketRead(socket_handle, head, 1, 10);

   // Header 255: Subscription Update (JSON)
   if(head[0] == 255) {
      uchar len_bytes[4];
      SocketRead(socket_handle, len_bytes, 4, 10);
      uint len = len_bytes[0]|len_bytes[1]<<8|len_bytes[2]<<16|len_bytes[3]<<24;
      uchar data[];
      ArrayResize(data, len);
      SocketRead(socket_handle, data, len, 500);
      UpdateSubscriptions(CharArrayToString(data));
   }
   // Header 100: Manual Sync Request (Symbol:HWM)
   else if(head[0] == 100) {
      uchar payload[64];
      SocketRead(socket_handle, payload, 64, 10);
      // Logic to parse "SYMBOL:HWM" and call SyncSymbol
      // (Simplified for now)
   }
}

void UpdateSubscriptions(string json) {
   ArrayFree(Subscriptions);
   // ... (Your existing JSON parsing logic to fill Subscriptions[]) ...
   // After updating, send sessions for the new list
   for(int i=0; i<ArraySize(Subscriptions); i++) {
      SymbolSelect(Subscriptions[i].symbol, true);
      SendSessionDefinitions(Subscriptions[i].symbol);
   }
}

// --- Event Handlers ---

int OnInit() {
   // Use 100ms for high-precision tick capture
   EventSetMillisecondTimer(100);
   return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason) {
   EventKillTimer();
   if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
}

void OnTimer() {
   if(!EnsureConnection()) return;

   // 1. Capture Ticks
   PollMarketData();

   // 2. Listen for Rust Instructions
   CheckForIncomingCommands();

   // 3. Heartbeat & Calibration (Every 30s)
   static uint last_cal = 0;
   if(GetTickCount() - last_cal > 30000) {
      SendCalibration();
      last_cal = GetTickCount();
   }
}