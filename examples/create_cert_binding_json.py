import argparse
import json


def main():
    parser = argparse.ArgumentParser(description="Create a JSON cert binding file")
    parser.add_argument("hostname")
    parser.add_argument("-c", "--cert", required=True)
    parser.add_argument("-k", "--key", required=True)
    parser.add_argument("-o", "--output", required=True)
    args = parser.parse_args()

    with open(args.cert, 'r') as file:
        cert = file.read()

    with open(args.key, 'r') as file:
        key = file.read()

    binding = {"host": args.hostname, "cert": cert, "key": key}
    with open(args.output, 'w') as file:
        json.dump(binding, file, indent=4)


if __name__ == "__main__":
    main()
