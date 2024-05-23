import argparse
import json


def main():
    parser = argparse.ArgumentParser(description="Create a JSON cert binding file")
    parser.add_argument("--host", required=True, help="Hostname to bind the cert to")
    parser.add_argument("-c", "--cert", required=True, help="Certificate file")
    parser.add_argument("-k", "--key", required=True, help="Key file")
    parser.add_argument("-o", "--output", required=True, help="Output JSON file")
    args = parser.parse_args()

    with open(args.cert, 'r') as file:
        cert = file.read()

    with open(args.key, 'r') as file:
        key = file.read()

    binding = {"host": args.host, "cert": cert, "key": key}
    with open(args.output, 'w') as file:
        json.dump(binding, file, indent=4)


if __name__ == "__main__":
    main()
