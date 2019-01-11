#!/usr/bin/env python3


import sys
import json
import argparse
import functools
import collections


stderr_print = functools.partial(print, file=sys.stderr)


class Converter:
    MAIN = ('network_id', 'network_name', 'provider', 'nit_version', 'onid', 'textcode', 'country', 'offset')
    MAIN_MAP = {
        'network_name': 'network',
        'textcode': 'codepage'
    }
    MULTIPLEX = ('name', 'tsid', 'enable')

    CAST = {
        bool: int
    }

    def __init__(self, *configs):
        """
        :param configs: Sequence of config files paths.
        """

        self.configs = configs
        self.header = collections.OrderedDict()
        self.header.filled = False

    def process(self):
        """
        Processing configuration files.
        """

        for f in self._fd_iter():
            try:
                data = json.loads(f.read(), object_pairs_hook=collections.OrderedDict)
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

                for i in self.MULTIPLEX:
                    try:
                        multiplex[i] = item[i]
                    except KeyError:
                        pass

                for i in self.MAIN:
                    try:
                        k = self.MAIN_MAP.get(i, i)
                        v = item[i]
                        if not self.header.filled:
                            self.header[k] = v
                        else:
                            if self.header[k] != v:
                                multiplex[k] = v
                    except KeyError:
                        pass

                if not self.header.filled:
                    self.header.filled = True
                    self.write(self.header)

                self.write(multiplex, 'multiplex')

                for service in item.get('sdt', ()):
                    self.write(service, 'service')

    def write(self, data, section=None):
        """
        Write section to stdout.

        :type data: dict
        :param data: Section payload.
        :type section: str
        :param section: Section name.
        """

        if section:
            print('[{}]'.format(section))

        for k, v in data.items():
            try:
                v = self.CAST[type(v)](v)
            except KeyError:
                pass
            print('{} = {}'.format(k, v))
        print()

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
