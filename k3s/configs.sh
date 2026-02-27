#!/bin/bash
set -e

# Namespace
kubectl create namespace techietenka --dry-run=client -o yaml | kubectl apply -f -
kubectl config set-context --current --namespace=techietenka

# Secrets & ConfigMaps
kubectl create secret generic techietenka-env-vars \
  --from-env-file=prod.env \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create configmap mosquitto-config \
  --from-file=mosquitto/config/mosquitto.conf \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic mosquitto-pwfile \
  --from-file=mosquitto/config/pwfile \
  --dry-run=client -o yaml | kubectl apply -f -
