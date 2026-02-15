import socket

server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.bind(('127.0.0.1', 9090))
server.listen(1)
print("📡 Waiting for MT5 on port 9090...")

conn, addr = server.accept()
print(f"✅ MT5 Connected from {addr}")

while True:
    data = conn.recv(1024)
    if not data: break
    print(f"📈 Received: {data.decode('utf-8').strip()}")