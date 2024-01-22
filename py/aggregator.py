import tensorflow as tf
import numpy as np
import toml
import sys
import os
import random as rd
from bridge import Bridge

class Aggregator:
    def __init__(self, host, port):
        self.bridge = Bridge(host, port)
        self.bridge.connect()
        self.model = None
        self.trainers = []
        self.path = './model.toml'
    
    def aggregate(self, ws):
        if [len(w) for w in ws] != [len(ws[0])] * len(ws):
            exit(1)
        avg_weights = [np.array([w[i] for w in ws]).mean(axis=0) for i in range(len(ws[0]))]
        self.model.set_weights(avg_weights)
    
    def run(self, subset, epochs, cycles):
        if self.model is not None:
            for _ in range(cycles):
                trs = rd.sample(self.trainers, k = int(len(self.trainers) * subset))

                bin_model = toml.dumps({'epochs': epochs, 'model': self.model.to_json(), 'weights': self.model.get_weights()}, encoder=toml.TomlNumpyEncoder()).encode()

                send = {'sends': []}
                for i in range(len(trs)):
                    send['sends'].append({'topic': trs[i] + ' train model', 'data': bin_model, 'expect': {'topic': 'aggregator model trained ' + trs[i], 'recv': {'id': str(i), 'interest': r'^aggregator model trained ' + trs[i] + r'$', 'num': 1}}})
                self.bridge.send(toml.dumps(send))
                print(f'message sent')

                msg = self.bridge.receive()
                if msg is None:
                    exit(1)
                ress = toml.loads(msg)['ress']
                print('message received')

                weights = list(map(lambda x: toml.loads(bytes(x['packets'][0]['data']).decode())['weights'], ress))

                self.aggregate(weights)

                f = open(self.path, 'w')
                f.write(toml.dumps({'model': self.model.to_json(), 'weights': self.model.get_weights()}, encoder=toml.TomlNumpyEncoder()))
                f.close()


if __name__ == '__main__':
    host = '127.0.0.1' # sys.argv[1]
    port = '8000' # sys.argv[2]
    aggr = Aggregator(host, port)
    aggr.model = tf.keras.models.Sequential([
        tf.keras.layers.Flatten(input_shape=(28, 28)),
        tf.keras.layers.Dense(128, activation='relu'),
        tf.keras.layers.Dense(10)
    ])
    aggr.model.compile(
        optimizer=tf.keras.optimizers.Adam(0.001),
        loss=tf.keras.losses.SparseCategoricalCrossentropy(from_logits=True),
        metrics=[tf.keras.metrics.SparseCategoricalAccuracy()],
    )
    aggr.trainers = ['alice', 'bob', 'charlie'][:-1]

    os.system('clear')

    while True:
        subset = float(input('subset: '))
        epochs = int(input('epochs: '))
        cycles = int(input('cylces: '))
        aggr.run(subset, epochs, cycles)
        print('')
