import tensorflow as tf
import tensorflow_datasets as tfds
import numpy as np
import toml
import sys
import os
from bridge import Bridge

class Trainer:
    def __init__(self, host, port):
        self.bridge = Bridge(host, port)
        self.model = None
        self.train_set = None
        self.test_set = None
        self.info = None

    def normalize_train_set(self):
        self.train_set = self.train_set.map(Trainer.normalize, num_parallel_calls=tf.data.AUTOTUNE)
        self.train_set = self.train_set.cache()
        self.train_set = self.train_set.shuffle(self.info.splits['train'].num_examples)
        self.train_set = self.train_set.batch(128)
        self.train_set = self.train_set.prefetch(tf.data.AUTOTUNE)

    def normalize_test_set(self):
        self.test_set = self.test_set.map(Trainer.normalize, num_parallel_calls=tf.data.AUTOTUNE)
        self.test_set = self.test_set.batch(128)
        self.test_set = self.test_set.cache()
        self.test_set = self.test_set.prefetch(tf.data.AUTOTUNE)

    def normalize(sample, label):
       return tf.cast(sample, tf.float32) / 255., label
    
    def train(self, epochs):
        self.model.compile(
            optimizer=tf.keras.optimizers.Adam(0.001),
            loss=tf.keras.losses.SparseCategoricalCrossentropy(from_logits=True),
            metrics=[tf.keras.metrics.SparseCategoricalAccuracy()],
        )
        self.model.fit(self.train_set, epochs=epochs, validation_data=self.test_set)
    
    def run(self, topic, num, tot):
        (self.train_set, self.test_set), self.info = tfds.load('mnist', split=[f'train[{(num - 1) * 100 / tot}%:{num * 100 / tot}%]', f'test[{(num - 1) * 100 / tot}%:{num * 100 / tot}%]'], shuffle_files=True, as_supervised=True, with_info=True)
        self.normalize_train_set()
        self.normalize_test_set()
        os.system('clear')

        self.bridge.connect()
        print('bridge connected')

        self.bridge.send(toml.dumps({'recvs': [{'id': '0', 'interest': f'{topic} train model', 'num': 0}]}))
        print('listener published')

        while (msg := self.bridge.receive()) != None:
            print(f'message received')
            obj = toml.loads(msg)
            pkt = obj['ress'][0]['packets'][0]
            data = bytes(pkt['data']).decode()
            toml_model = toml.loads(data)

            epochs = toml_model['epochs']
            json_model = toml_model['model']
            weights = [np.array(w) for w in toml_model['weights']]

            self.model = tf.keras.models.model_from_json(json_model)
            self.model.set_weights(weights)

            self.train(epochs)

            bin_weights = toml.dumps({'weights': self.model.get_weights()}, encoder=toml.TomlNumpyEncoder()).encode()

            req = {'sends': [{'topic': pkt['topic'], 'data': bin_weights}]}

            self.bridge.send(toml.dumps(req))
            print('message sent')

if __name__ == '__main__':
    host = sys.argv[1]
    port = sys.argv[2]
    num = sys.argv[3]
    tot = sys.argv[4]
    topic = sys.argv[5]
    trainer = Trainer(host, port)
    trainer.run(topic, int(num), int(tot))