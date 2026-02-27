openssl genrsa -out /etc/ssl/private/acl_rsa_token.key 2048
openssl rsa -in /etc/ssl/private/acl_rsa_token.key -pubout -out /etc/ssl/certs/acl_rsa_token_pub.pem

kubectl create secret docker-registry registry-auth \
    --docker-server=registry.techietenka.com \
    --docker-username=YOUR_USERNAME \
    --docker-password=YOUR_PASSWORD \
    -n techietenka

kubectl create secret tls techietenka-main-tls \
       --cert=/etc/ssl/certs/techietenka_fullchain.crt \
       --key=/etc/ssl/private/techietenka.key \
       -n techietenka

kubectl create secret generic acl-rsa-keys \
  --from-file=/etc/ssl/private/acl_rsa_token.key \
  --from-file=/etc/ssl/certs/acl_rsa_token_pub.pem \
  -n techietenka
