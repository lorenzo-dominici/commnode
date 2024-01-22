import socket
import struct
import toml
import time

class Bridge:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    def connect(self):
        self.socket.connect((self.host, int(self.port)))

    def send(self, message):
        try:
            print(f'\x1b[96mSEND\x1b[0m [{time.time_ns()}] ', end='')
            # Prefix the message with its length as a 4-byte little-endian binary data
            message_length = len(message)
            length_prefix = struct.pack('<I', message_length)
            size = len(length_prefix + message.encode())
            self.socket.sendall(length_prefix + message.encode())
            print(f'... [{time.time_ns()}] = {size} Bytes')
        except Exception as e:
            pass

    def receive(self):
        try:
            print(f'\x1b[94mRECV\x1b[0m [{time.time_ns()}] ', end='')
            # Receive the length prefix (4 bytes, little-endian)
            length_prefix = self.socket.recv(4)
            if not length_prefix:
                return None

            # Unpack the length prefix to get the message length (little-endian)
            message_length = struct.unpack('<I', length_prefix)[0]
            
            data = b''

            while message_length > 0:
                # Receive the actual message
                d = self.socket.recv(message_length)
                if not d:
                    return None
                message_length -= len(d)
                data += d
            
            msg = data.decode()

            print(f'... [{time.time_ns()}] = {len(data) + 4} Bytes')

            return msg
        except Exception as e:
            return None

    def close(self):
        self.socket.close()

if __name__ == "__main__":
    bridge = Bridge("127.0.0.1", 9000)
    bridge.connect()
    msg = toml.dumps({'sends': [{'topic': 'test', 'data': b'payload', 'expect': {'topic': 'test', 'recv': {'id': '1234', 'interest': r'^test$', 'num': 1}}}]})
    print(msg)
    bridge.send(msg)
    msg = bridge.receive()
    obj = toml.loads(msg)
    obj['ress'][0]['packets'][0]['data'] = bytes(obj['ress'][0]['packets'][0]['data']).decode()
    print(obj)
    bridge.close()