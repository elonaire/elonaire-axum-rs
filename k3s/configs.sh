#!/bin/bash
kubectl create namespace techietenka
kubectl config set-context --current --namespace=techietenka
kubectl create secret generic techietenka-env-vars --from-env-file=prod.env
kubectl create configmap mosquitto-config --from-file=mosquitto/config/mosquitto.conf
kubectl create secret generic mosquitto-pwfile --from-file=mosquitto/config/pwfile
