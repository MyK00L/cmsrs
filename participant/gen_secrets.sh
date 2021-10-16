#!/bin/sh
# generate tls certs
mkdir -p certs
openssl req -newkey rsa:2048 \
    -nodes \
    -keyout certs/key.pem \
    -x509 \
    -days 365 \
    -out certs/certificate.pem \
    -subj "/C=IT/ST=/L=/O=/OU=/CN=/emailAddress="
# replace rocket secret
secret_key="\"$(openssl rand -base64 32)\""
perl -pi -e 's/^secret_key.*/secret_key='$secret_key'/' Rocket.toml
