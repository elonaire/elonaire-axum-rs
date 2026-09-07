#!/bin/bash
set -e

# ArgoCD
kubectl create namespace argocd --dry-run=client -o yaml | kubectl apply -f -
kubectl apply -n argocd --server-side --force-conflicts -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
kubectl patch deployment argocd-server -n argocd --type='json' -p='[{"op": "add", "path": "/spec/template/spec/containers/0/command/-", "value": "--insecure"}]'

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
