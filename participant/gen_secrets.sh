#!/bin/bash
# Generate tls certs.
mkdir -p certs
openssl req -newkey rsa:2048 \
    -nodes \
    -keyout certs/key.pem \
    -x509 \
    -days 365 \
    -out certs/certificate.pem \
    -subj "/C=IT/ST=/L=/O=/OU=/CN=/emailAddress="
# Replace rocket secret.
secret_key="\"$(openssl rand -base64 32)\""
# Use pipe ("|") character as sed delimiter since the base64 secret_key cannot contian such a char.
sed -i 's|^secret_key.*|secret_key='$secret_key'|' Rocket.toml
