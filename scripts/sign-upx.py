"""
Sign a UPX-compressed binary using Tauri's Ed25519 key.
Supports both Tauri v2 JSON key format and rsign encrypted key format.

Usage: python sign-upx.py <exe_path> <key_file> <password>
  key_file: path to file containing the private key (any format)
"""
import sys
import os
import base64
import json
import hashlib


def ensure_pynacl():
    """Ensure PyNaCl is installed."""
    try:
        import nacl  # noqa: F401
    except ImportError:
        print("Installing PyNaCl...", file=sys.stderr)
        import subprocess
        subprocess.check_call(
            [sys.executable, '-m', 'pip', 'install', 'pynacl', '--quiet', '--user'])


def parse_rsign_key(b64_data: bytes, password: str) -> bytes:
    """
    Parse rsign encrypted secret key format and return the 64-byte Ed25519 key.

    rsign format layout (149 bytes total):
    [1 byte:   version]
    [36 bytes: unauthenticated comment (4-byte LE length + 32 bytes text)]
    [16 bytes: salt]
    [80 bytes: encrypted key (64-byte key + 16-byte Poly1305 tag)]
    [16 bytes: BLAKE2b-160 MAC]
    """
    import nacl.bindings
    from nacl.pwhash import scrypt
    from nacl.secret import SecretBox

    payload = base64.b64decode(b64_data)

    # Extract components
    salt = payload[37:53]
    enc_data = payload[53:133]  # 80 bytes: 64 key + 16 tag
    mac = payload[133:149]

    # Derive key using scrypt (N=32768, r=8, p=1, dkLen=64)
    # Matches libsodium crypto_pwhash_SCRYPTSALSA208SHA256 with INTERACTIVE limits
    derived = scrypt.kdf(64, password.encode('utf-8'), salt,
                         opslimit=32768, memlimit=8, datalimit=1)

    # Verify MAC using generichash (BLAKE2b) with derived key
    computed_mac = nacl.bindings.crypto_generichash(
        enc_data, key=derived[:32], digest_size=16)
    if not nacl.bindings.sodium_memcmp(computed_mac, mac):
        print("ERROR: Wrong password or corrupted key (MAC verification failed)",
              file=sys.stderr)
        sys.exit(1)

    # Decrypt using SecretBox (XSalsa20-Poly1305 / XChaCha20-Poly1305)
    # rsign uses a zero nonce
    nonce = b'\x00' * SecretBox.NONCE_SIZE  # 24 bytes
    box = SecretBox(derived[:32])

    try:
        raw_key = box.decrypt(enc_data, nonce=nonce)
    except Exception as e:
        print(f"ERROR: Decryption failed - {e}", file=sys.stderr)
        print("Wrong password or corrupted key", file=sys.stderr)
        sys.exit(1)

    if len(raw_key) != 64:
        print(f"ERROR: Expected 64-byte key, got {len(raw_key)} bytes",
              file=sys.stderr)
        sys.exit(1)

    print(f"Key decrypted successfully ({len(raw_key)} bytes)")
    return raw_key


def generate_signature(exe_path: str, raw_key: bytes, sig_path: str):
    """Generate Ed25519 signature for the binary."""
    import nacl.signing

    # Ed25519 key is 64 bytes: [32-byte seed || 32-byte public key]
    seed = raw_key[:32]
    signing_key = nacl.signing.SigningKey(seed)

    # Read binary
    with open(exe_path, 'rb') as f:
        binary = f.read()

    # Sign
    signed = signing_key.sign(binary)
    signature = signed.signature

    # Write signature as base64
    with open(sig_path, 'w') as f:
        f.write(base64.b64encode(signature).decode('ascii'))

    print(f"Ed25519 signature generated ({len(signature)} bytes)")


def main():
    if len(sys.argv) < 4:
        print("Usage: python sign-upx.py <exe_path> <key_file> <password>",
              file=sys.stderr)
        sys.exit(1)

    exe_path = sys.argv[1]
    key_file = sys.argv[2]
    password = sys.argv[3]
    sig_path = exe_path + '.sig'

    # Read key file
    with open(key_file, 'r', encoding='utf-8') as f:
        key_content = f.read().strip()

    # Try Tauri v2 JSON format first (base64-encoded JSON)
    if not key_content.startswith('untrusted'):
        try:
            key_data = json.loads(base64.b64decode(key_content).decode('utf-8'))
            raw_key = base64.b64decode(key_data['key'])
            print(f"Key loaded from JSON format ({len(raw_key)} bytes)")
            generate_signature(exe_path, raw_key, sig_path)
            return
        except Exception as e:
            print(f"ERROR: Failed to parse JSON key format: {e}",
                  file=sys.stderr)
            sys.exit(1)

    # rsign encrypted format
    print("Detected rsign encrypted key format")

    # Find the base64: line
    lines = key_content.split('\n')
    b64_line = None
    for line in lines:
        if line.startswith('base64:'):
            b64_line = line[7:]  # Strip "base64:" prefix
            break

    if not b64_line:
        print("ERROR: Cannot find base64: line in rsign key", file=sys.stderr)
        sys.exit(1)

    ensure_pynacl()
    raw_key = parse_rsign_key(b64_line.encode('ascii'), password)
    generate_signature(exe_path, raw_key, sig_path)


if __name__ == '__main__':
    main()
