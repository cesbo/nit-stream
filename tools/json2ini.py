#!/usr/bin/env python3


import sys
import json
import argparse
import functools
import collections


inf = float('inf')
stderr_print = functools.partial(print, file=sys.stderr)


class Descriptor:
    def __init__(self, name, keys):
        """
        :type name: str
        :param name: Name of descriptor section.
        :type keys: Iterable
        :param keys: Sequence of keys.
        """

        self.name = name
        self.keys = keys

    def assemble(self, data):
        """
        Assembling data to write.

        :type data: Mapping
        :param data: Input data for processing.

        :rtype: collections.OrderedDict
        :return: Data to write.
        """

        keys = []
        values = []
        for k, d in self.keys:
            keys.append(k)
            v = data.get(k, d)
            if v is inf:
                raise KeyError(k)
            values.append(v)

        return collections.OrderedDict(
            zip(keys, values)
        )


class Converter:
    MAP = {
        'network_name': 'network',
        'textcode': 'codepage'
    }
    MAIN = ('network_id', 'network_name', 'provider', 'nit_version', 'onid', 'textcode', 'country', 'offset')
    MULTIPLEX = ('name', 'tsid', 'onid', 'textcode', 'provider', 'enable')
    DVB_C = (
        ('frequency', inf),
        ('symbolrate', inf),
        ('fec', 0),
        ('modulation', 0)
    )
    DVB_S = DVB_C + (
        ('polarization', inf),
        ('position', inf)
    )

    CAST = {
        bool: int
    }

    def __init__(self, *configs, output_stream=sys.stdout):
        """
        :type configs: Iterable
        :param configs: Sequence of config files paths.
        :type output_stream: TextIOBase
        :param output_stream: File-like object (stream) to result output.
        """

        self.configs = configs

        self.header = collections.OrderedDict()
        self.header.filled = False

        self._descriptors = {
            'S': Descriptor('dvb-s', self.DVB_S),
            'C': Descriptor('dvb-c', self.DVB_C)
        }
        self.__print = functools.partial(print, file=output_stream)

    def process(self):
        """
        Processing configuration files.
        """

        for f in self._fd_iter():
            try:
                data = json.load(f, object_pairs_hook=collections.OrderedDict)
            except json.JSONDecodeError as e:
                stderr_print(e)
                sys.exit(1)

            stream = data.get('make_stream')
            if not stream:
                stderr_print('{} does not have "make_stream" section.'.format(f.name))
                sys.exit(1)

            for item in stream:
                multiplex = collections.OrderedDict()

                if item['type'] != 'mpts':
                    continue

                if not self.header.filled:
                    for i in self.MAIN:
                        try:
                            self.header[self.MAP.get(i, i)] = item[i]
                        except KeyError:
                            pass
                    self.header.filled = True
                    self.write(self.header)

                for i in self.MULTIPLEX:
                    try:
                        k = self.MAP.get(i, i)
                        v = item[i]
                        if self.header.get(k) != v:
                            multiplex[k] = v
                    except KeyError:
                        pass

                self.write(multiplex, 'multiplex')

                nit = item.get('nit_actual')
                if nit:
                    descriptor = self._descriptors.get(nit['type'])
                    if descriptor:
                        self.write(
                            descriptor.assemble(nit),
                            descriptor.name
                        )

                for service in item.get('sdt', ()):
                    self.write(service, 'service')

    def write(self, data, section=None):
        """
        Write section to stdout.

        :type data: Mapping
        :param data: Section payload.
        :type section: str
        :param section: Section name.
        """

        if section:
            self.__print('[{}]'.format(section))

        for k, v in data.items():
            try:
                v = self.CAST[type(v)](v)
            except KeyError:
                pass
            self.__print('{} = {}'.format(k, v))
        self.__print()

    def _fd_iter(self):
        for config in self.configs:
            try:
                with open(config) as f:
                    yield f
            except Exception as e:
                stderr_print(e)
                sys.exit(1)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Convert astra config to ini format.')
    parser.add_argument('config', nargs='+', help='path to config file')

    Converter(*parser.parse_args().config).process()
