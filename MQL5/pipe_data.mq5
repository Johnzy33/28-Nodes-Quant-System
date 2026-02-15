//+------------------------------------------------------------------+
//|                                                    pipe_data.mq5 |
//|                                   Copyright 2026, Gemini Thought |
//|                                             https://www.mql5.com |
//+------------------------------------------------------------------+
#property version   "1.10"
#property strict

// --- Data Structures ---

struct MultiplexedTick {
   char   asset_name[10]; 
   char   _dummy[6];      // Padding for 8-byte alignment
   double bid; 
   double ask; 
   double last;           
   long   volume; 
   long   time_msc;       
   uint   flags; 
   uint   _end_pad;       
}; 


struct MarketTick {
   uint     symbol_id; // 4 bytes
   // 4 bytes of padding added by compiler to align double
   double   bid;       // 8 bytes
   double   ask;       // 8 bytes
   ulong    volume;    // 8 bytes
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
   char     asset[16];      // Symbol Name
   uint     open_hour;
   uint     open_min;
   uint     close_hour;
   uint     close_min;
   uint     is_active;      // 1 = Open today, 0 = Holiday/Closed
};

struct SubscribedAsset {
   string symbol;
   ENUM_TIMEFRAMES timeframe;
   long last_sync;
   double last_sent_bid;    // For stale data filtering
};


// --- Global Variables ---

SubscribedAsset Subscriptions[];
int socket_handle = INVALID_HANDLE;
bool is_syncing = false;

// --- Communication Helpers ---

template<typename T>
void SafeSend(uchar header_type, T &struct_obj) {
   if(socket_handle == INVALID_HANDLE) return;
   uchar header[1] = {header_type};
   uchar payload[64]; 
   ZeroMemory(payload);
   uchar raw_data[]; 
   StructToCharArray(struct_obj, raw_data);
   ArrayCopy(payload, raw_data, 0, 0, MathMin(ArraySize(raw_data), 64));
   
   SocketSend(socket_handle, header, 1);
   SocketSend(socket_handle, payload, 64);
}

// --- Negotiation Logic ---

bool RustNeedsSync(string symbol) {
   if(socket_handle == INVALID_HANDLE) return true;
   
   uchar header[1] = {5}; // HEADER_NEGOTIATE
   uchar payload[64];
   ZeroMemory(payload);
   StringToCharArray(symbol, payload, 0, 10);
   
   // Send Negotiation Request
   SocketSend(socket_handle, header, 1);
   SocketSend(socket_handle, payload, 64);
   
   // Wait for 1-byte response (1 = Live/Skip, 0 = Needs Sync)
   uchar response[]; // Use a dynamic array for SocketRead
   ArrayResize(response, 1);
   uint start_time = GetTickCount();
   
   // Wait until at least 1 byte is available or timeout occurs
   while(SocketIsReadable(socket_handle) < 1 && GetTickCount() - start_time < 1000) Sleep(10);
   
   // FIXED: Replaced SocketRecv with SocketRead
   if(SocketRead(socket_handle, response, 1, 100) > 0) {
      if(response[0] == 1) {
         Print("✅ Rust Engine reports ", symbol, " is already LIVE. Skipping redundant sync.");
         return false;
      }
   }
   
   Print("📥 Rust Engine reports ", symbol, " is NEW. Initiating history sync...");
   return true;
}

// --- Connection Logic ---

bool EnsureConnection() {
   if(socket_handle == INVALID_HANDLE || !SocketIsConnected(socket_handle)) {
      if(socket_handle != INVALID_HANDLE) SocketClose(socket_handle);
      socket_handle = SocketCreate();
      
      if(SocketConnect(socket_handle, "127.0.0.1", 9090, 500)) {
         // 1. Send Calibration Time (Header 254)
         uchar header[1] = {254};
         long current_broker_time = TimeCurrent();
         uchar payload[8]; 
         for(int i=0; i<8; i++) payload[i] = (uchar)((current_broker_time >> (i*8)) & 0xFF);
         SocketSend(socket_handle, header, 1);
         SocketSend(socket_handle, payload, 8);
         
         Print("📡 Reconnected & Calibrated.");
         return true;
      }
      return false;
   }
   return true;
}

// --- Session Reporting ---

void SendSessionUpdate(string symbol) {
   MqlDateTime dt_struct;
   datetime now_server = TimeCurrent(dt_struct);
   ENUM_DAY_OF_WEEK day = (ENUM_DAY_OF_WEEK)dt_struct.day_of_week;
   
   datetime start, end;
   SessionPacket packet;
   ZeroMemory(packet);
   StringToCharArray(symbol, packet.asset, 0, 16);
   
   // 1. Check if a session is defined for today
   if(SymbolInfoSessionQuote(symbol, day, 0, start, end)) {
      
      // SymbolInfoSessionQuote returns seconds since 00:00:00 of the day.
      // We need to see if 'now' is actually between those seconds.
      long seconds_since_midnight = (dt_struct.hour * 3600) + (dt_struct.min * 60) + dt_struct.sec;
      
      // Formatting for the Print statement
      string start_str = IntegerToString(start/3600, 2, '0') + ":" + IntegerToString((start%3600)/60, 2, '0');
      string end_str   = IntegerToString(end/3600, 2, '0') + ":" + IntegerToString((end%3600)/60, 2, '0');
      string now_str   = IntegerToString(dt_struct.hour, 2, '0') + ":" + IntegerToString(dt_struct.min, 2, '0');

      // 2. CRITICAL: Only mark active if we are actually INSIDE the session time
      if(seconds_since_midnight >= (long)start && seconds_since_midnight < (long)end) {
         packet.is_active = 1;
         Print("🟢 ", symbol, " ACTIVE. Session: ", start_str, "-", end_str, " (Now: ", now_str, ")");
      } else {
         packet.is_active = 0;
         Print("🟡 ", symbol, " CLOSED (Out of Hours). Session: ", start_str, "-", end_str, " (Now: ", now_str, ")");
      }
      
      packet.open_hour = (uint)(start / 3600);
      packet.open_min  = (uint)((start % 3600) / 60);
      packet.close_hour = (uint)(end / 3600);
      packet.close_min  = (uint)((end % 3600) / 60);
      
   } else {
      packet.is_active = 0; 
      Print("💤 ", symbol, " INACTIVE. No session defined for ", EnumToString(day));
   }
   
   // 3. Send the packet to Rust
   uchar header[1] = {3};
   SocketSend(socket_handle, header, 1);
   uchar raw_packet[];
   StructToCharArray(packet, raw_packet);
   SocketSend(socket_handle, raw_packet, sizeof(SessionPacket));
}

// --- Ingestion Logic ---

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
   static int last_day = -1;
   MqlDateTime now_dt;
   TimeCurrent(now_dt);
   
   if(now_dt.day != last_day) {
      last_day = now_dt.day;
      for(int i=0; i<ArraySize(Subscriptions); i++) {
         SendSessionUpdate(Subscriptions[i].symbol);
      }
   }
   
   if(is_syncing || !EnsureConnection()) return;

   uint readable = SocketIsReadable(socket_handle);
   if(readable >= 5) {
      uchar head[5];
      SocketRead(socket_handle, head, 5, 10);
      if(head[0] == 255) {
         uint len = head[1]|head[2]<<8|head[3]<<16|head[4]<<24;
         uchar data[]; ArrayResize(data, len);
         SocketRead(socket_handle, data, len, 500);
         ParseSubscription(CharArrayToString(data));
         return;
      }
   }

   for(int i=0; i<ArraySize(Subscriptions); i++) {
   
      string sym = Subscriptions[i].symbol;
      // MASTER GATE: Check if the broker has a quote session for right now
      datetime s, e;
      if(!SymbolInfoSessionQuote(sym, (ENUM_DAY_OF_WEEK)now_dt.day_of_week, 0, s, e)) {
         continue; // It is a blank day (Sat/Sun); skip tick processing entirely
      }
      
      MqlTick tick;
      if(SymbolInfoTick(Subscriptions[i].symbol, tick)) {
         if(tick.bid == Subscriptions[i].last_sent_bid) continue;

         MultiplexedTick p; 
         ZeroMemory(p);
         StringToCharArray(Subscriptions[i].symbol, p.asset_name, 0, 10);
         p.bid = tick.bid; 
         p.ask = tick.ask; 
         p.time_msc = tick.time_msc;
         
         SafeSend(0, p);
         Subscriptions[i].last_sent_bid = tick.bid;
      }
   }
}

// --- Subscription & Sync Logic ---

void ParseSubscription(string json) {
   ArrayFree(Subscriptions);
   string items[];
   int total = StringSplit(json, '}', items);
   for(int i=0; i<total; i++) {
      string sym = ExtractValue(items[i], "mt5");
      if(sym == "") continue;
      
      int idx = ArraySize(Subscriptions);
      ArrayResize(Subscriptions, idx+1);
      Subscriptions[idx].symbol = sym;
      Subscriptions[idx].timeframe = StringToTF(ExtractValue(items[i], "tf"));
      Subscriptions[idx].last_sent_bid = 0;
      
      SymbolSelect(sym, true);
      
      // Send session info immediately
      SendSessionUpdate(sym);
      
      // NEGOTIATION: Check with Rust before Syncing
      if(RustNeedsSync(sym)) {
         is_syncing = true;
         SyncSymbol(Subscriptions[idx], (long)ExtractValue(items[i], "hwm"));
         is_syncing = false;
      }
   }
}

void SyncSymbol(SubscribedAsset &asset, long hwm) {
   MqlRates rates[];
   // Calculate HWM (Database is UTC, MT5 CopyRates uses Broker Time)
   int copied = CopyRates(asset.symbol, asset.timeframe, (datetime)hwm, TimeCurrent(), rates);
   for(int i=0; i<copied; i++) {
      SyncBar b; ZeroMemory(b);
      StringToCharArray(asset.symbol, b.asset, 0, 10);
      b.open=rates[i].open; b.high=rates[i].high; b.low=rates[i].low; b.close=rates[i].close;
      b.volume=(long)rates[i].tick_volume; b.time=(long)rates[i].time;
      SafeSend(1, b);
   }
}

// --- Utility Functions ---

string ExtractValue(string text, string key) {
   int pos = StringFind(text, "\""+key+"\"");
   if(pos < 0) return "";
   int start = StringFind(text, ":", pos) + 1;
   int end = StringFind(text, ",", start);
   if(end < 0) end = StringFind(text, "]", start);
   if(end < 0) end = StringFind(text, "}", start);
   string res = StringSubstr(text, start, end-start);
   StringReplace(res, "\"", "");
   StringReplace(res, "{", "");
   StringReplace(res, "}", "");
   StringReplace(res, ":", "");
   StringTrimLeft(res);
   StringTrimRight(res);
   return res;
}

ENUM_TIMEFRAMES StringToTF(string tf) {
   if(tf == "M1") return PERIOD_M1;
   if(tf == "M5") return PERIOD_M5;
   if(tf == "M15") return PERIOD_M15;
   if(tf == "H1") return PERIOD_H1;
   return PERIOD_M15;
}