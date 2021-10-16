# delete tls certs
rm -rf certs
# replace rocket secret
secret_key="\"\""
sed -i "s/^secret_key.*/secret_key=${secret_key}/" Rocket.toml

