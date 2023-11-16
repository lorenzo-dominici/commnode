import socket
import struct

class TcpBridge:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    def connect(self):
        self.socket.connect((self.host, self.port))

    def send(self, message):
        try:
            # Prefix the message with its length as a 4-byte little-endian binary data
            message_length = len(message)
            length_prefix = struct.pack('<I', message_length)
            self.socket.sendall(length_prefix + message.encode())
        except Exception as e:
            pass

    def receive(self):
        try:
            # Receive the length prefix (4 bytes, little-endian)
            length_prefix = self.socket.recv(4)
            if not length_prefix:
                return None

            # Unpack the length prefix to get the message length (little-endian)
            message_length = struct.unpack('<I', length_prefix)[0]

            # Receive the actual message
            data = self.socket.recv(message_length)
            if not data:
                return None

            return data.decode()
        except Exception as e:
            return None

    def close(self):
        self.socket.close()

if __name__ == "__main__":
    bridge = TcpBridge("127.0.0.1", 9000)
    bridge.connect()
    bridge.send(
'''
[[sends]]
topic = "test"
data = "payload"

[sends.expect]
id = "1234"
interest = "^test$"
num = 1
''')
    msg = bridge.receive()
    print(msg)
    bridge.close()