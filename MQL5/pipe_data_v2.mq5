
#property copyright "Copyright 2026, 28th Node Systems"
#property link      "28thNodesystem.com"
#property version   "2.10"
#property strict

// --- Binary Aligned Structs (Matched to Rust) ---

struct MultiplexedTick {
   char   asset[10];
   char   _pad[6];         
   double bid;
   double ask;
   double last;
   long   volume;
   long   time_msc;
   uint   flags;
   uint   _end_pad;        // Total: 64 bytes
};

struct SyncBar {
   char   asset[10];
   char   _pad[6];
   double open;
   double high;
   double low;
   double close;
   long   volume;
   long   time;            // Total: 64 bytes
};

struct SessionPacket {
   char   asset[16];
   uint   open_hour;
   uint   open_min;
   uint   close_hour;
   uint   close_min;
   uint   is_active;       // Total: 36 bytes
};

struct SubscribedAsset {
   string symbol;
   double last_sent_bid;
   double last_sent_ask;
};

// --- Globals ---
SubscribedAsset Subscriptions[];
int socket_handle = INVALID_HANDLE;
uint last_reconnect = 0;

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
   if(GetTickCount() - last_reconnect < 5000) return false;
   last_reconnect = GetTickCount();

   if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
   socket_handle = SocketCreate();
   if(SocketConnect(socket_handle, "127.0.0.1", 9090, 100)) {
      Print("📡 Rust Engine Online.");
      // Initial Sync: Send sessions
      for(int i=0; i<ArraySize(Subscriptions); i++) SendSessionDefinitions(Subscriptions[i].symbol);
      return true;
   }
   return false;
}

// --- Logic Modules ---

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

void SyncSymbolChunked(string symbol, datetime from_ts) {
   MqlRates rates[];
   // Request M15 bars from from_ts to now
   int copied = CopyRates(symbol, PERIOD_M15, from_ts, TimeCurrent(), rates);
   
   if(copied <= 0) return;

   Print("📦 Syncing ", copied, " bars for ", symbol);

   for(int i=0; i<copied; i++) {
      SyncBar bar;
      ZeroMemory(bar);
      StringToCharArray(symbol, bar.asset, 0, 10);
      bar.open = rates[i].open;
      bar.high = rates[i].high;
      bar.low = rates[i].low;
      bar.close = rates[i].close;
      bar.volume = (long)rates[i].tick_volume;
      bar.time = rates[i].time;

      SafeSend(1, bar); // Header 1: History Bar

      // Every 100 bars, take a tiny break to let live ticks through
      if(i % 100 == 0) Sleep(1); 
   }
   Print("✅ Sync complete for ", symbol);
}

void CheckForIncomingCommands() {
   if(socket_handle == INVALID_HANDLE || !SocketIsReadable(socket_handle)) return;

   uchar head[1];
   if(SocketRead(socket_handle, head, 1, 10) < 1) return;

   // Header 100: Manual Sync Request (16 bytes Asset + 8 bytes Timestamp)
   if(head[0] == 100) {
      uchar name_buf[16];
      uchar ts_buf[8];
      
      if(SocketRead(socket_handle, name_buf, 16, 10) == 16 && 
         SocketRead(socket_handle, ts_buf, 8, 10) == 8) {
         
         string sym = CharArrayToString(name_buf);
         long ts = 0;
         for(int i=0; i<8; i++) ts |= (long)ts_buf[i] << (8 * i);
         
         SyncSymbolChunked(sym, (datetime)ts);
      }
   }
}

void PollMarketData() {
   for(int i=0; i<ArraySize(Subscriptions); i++) {
      MqlTick t;
      if(SymbolInfoTick(Subscriptions[i].symbol, t)) {
         if(t.bid == Subscriptions[i].last_sent_bid && t.ask == Subscriptions[i].last_sent_ask) continue;

         MultiplexedTick p;
         ZeroMemory(p);
         StringToCharArray(Subscriptions[i].symbol, p.asset, 0, 10);
         p.bid = t.bid;
         p.ask = t.ask;
         p.last = t.last;
         p.volume = (long)t.tick_volume;
         p.time_msc = t.time_msc;
         p.flags = t.flags;
         
         SafeSend(0, p); // Header 0: Live Tick
         Subscriptions[i].last_sent_bid = t.bid;
         Subscriptions[i].last_sent_ask = t.ask;
      }
   }
}

// --- Event Handlers ---

int OnInit() {
   // Add your subscription logic here or via Rust JSON command
   // ArrayResize(Subscriptions, ...);
   
   EventSetMillisecondTimer(100);
   return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason) {
   EventKillTimer();
   if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
}

void OnTimer() {
   if(!EnsureConnection()) return;

   PollMarketData();
   CheckForIncomingCommands();

   // Calibration & Heartbeat every 30s
   static uint last_cal = 0;
   if(GetTickCount() - last_cal > 30000) {
      long cal[2] = {TimeCurrent(), TimeLocal()};
      SafeSend(254, cal);
      last_cal = GetTickCount();
   }
}