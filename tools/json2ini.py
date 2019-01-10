#!/usr/bin/env python3


import sys
import json
import argparse
import collections
import configparser


def stderr_print(*args, file=sys.stderr, **kwargs):
    """
    Print to stderr by default.
    """

    print(*args, file=file, **kwargs)


class Converter:
    KEY_MAP = {
        'network_name': 'network',
        'textcode': 'codepage'
    }
    MAIN = ('network_id', 'network_name', 'provider', 'nit_version', 'onid', 'textcode', 'country', 'offset')
    MULTIPLEX = ('enable', 'name', 'tsid')

    def __init__(self, *configs):
        self.configs = configs
        self.output = configparser.ConfigParser()
        self.header = None

    def process(self):
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

            for item in data:
                if item['type'] != 'mpts':
                    continue

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
