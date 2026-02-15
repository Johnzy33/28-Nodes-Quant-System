
//+------------------------------------------------------------------+
//|                                                    pipe_data.mq5 |
//|                                          Copyright 2025, 28th-Co |
//|                                             https://www.mql5.com |
//+------------------------------------------------------------------+

#property strict
#property version   "6.00"

// --- STACK-ALIGNED STRUCTURES (Exactly 64 Bytes) ---
struct MultiplexedTick {
   char   asset_name[10]; char _dummy[6];      
   double bid; double ask; double last;           
   long   volume; long time_msc;       
   uint   flags; uint _end_pad;       
}; // 10+6 + 8+8+8 + 8+8 + 4+4 = 64 bytes

struct SyncBar {
   char   asset[10]; char _pad[6];
   double open; double high; double low; double close;
   long   volume; long time; 
}; // 10+6 + 8+8+8+8 + 8+8 = 64 bytes

input string InpHost = "127.0.0.1";
input int    InpPort = 9090;

int  socket_handle = INVALID_HANDLE;
bool is_syncing    = false; 

// Template to safely map any struct to a 64-byte wire payload
template<typename T>
void SafeSend(uchar header_type, T &struct_obj) {
   if(socket_handle == INVALID_HANDLE) return;
   
   uchar header[1] = {header_type};
   uchar payload[64]; 
   ZeroMemory(payload);
   
   uchar raw_data[];
   StructToCharArray(struct_obj, raw_data);
   
   int to_copy = MathMin(ArraySize(raw_data), 64);
   ArrayCopy(payload, raw_data, 0, 0, to_copy);
   
   SocketSend(socket_handle, header, 1);
   SocketSend(socket_handle, payload, 64);
}

int OnInit() {
   socket_handle = SocketCreate();
   EventSetTimer(1); 
   return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason) {
   EventKillTimer();
   if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
}

void OnTimer() {
   if(is_syncing || !EnsureConnection()) return;

   uint readable = SocketIsReadable(socket_handle);
   if(readable >= 8) {
      uchar cmd_buf[8];
      if(SocketRead(socket_handle, cmd_buf, 8, 100) == 8) {
         // Drain extra bytes
         uint extra = SocketIsReadable(socket_handle);
         if(extra > 0) { uchar junk[]; ArrayResize(junk, extra); SocketRead(socket_handle, junk, extra, 10); }

         long start_ts = 0;
         for(int i=0; i<8; i++) start_ts |= ((long)cmd_buf[i] << (i * 8));
         
         is_syncing = true; 
         SendBarHistory(start_ts);
         is_syncing = false;
      }
   }
}

void OnTick() {
   if(is_syncing || !EnsureConnection()) return;

   MqlTick last_tick;
   if(SymbolInfoTick(_Symbol, last_tick)) {
      MultiplexedTick packet;
      ZeroMemory(packet);
      StringToCharArray(_Symbol, packet.asset_name, 0, 10);
      packet.bid      = last_tick.bid;
      packet.ask      = last_tick.ask;
      packet.last     = last_tick.last;
      packet.volume   = (long)last_tick.volume;
      packet.time_msc = last_tick.time_msc;
      packet.flags    = last_tick.flags;
      
      SafeSend(0, packet);
   }
}

void SendBarHistory(long start_timestamp) {
   MqlRates rates[];
   int copied = CopyRates(_Symbol, PERIOD_M15, (datetime)start_timestamp, TimeCurrent(), rates);
   if(copied <= 0) return;

   Print("📦 Syncing ", copied, " bars...");
   for(int i=0; i<copied; i++) {
      SyncBar bar;
      ZeroMemory(bar);
      StringToCharArray(_Symbol, bar.asset, 0, 10);
      bar.open   = rates[i].open; bar.high = rates[i].high;
      bar.low    = rates[i].low;  bar.close = rates[i].close;
      bar.volume = rates[i].tick_volume; bar.time = (long)rates[i].time;

      SafeSend(1, bar);
      Sleep(1); 
   }
   Print("✅ Sync Complete.");
}

bool EnsureConnection() {
   if(socket_handle == INVALID_HANDLE || !SocketIsConnected(socket_handle)) {
      if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
      socket_handle = SocketCreate();
      return SocketConnect(socket_handle, InpHost, InpPort, 500);
   }
   return true;
}