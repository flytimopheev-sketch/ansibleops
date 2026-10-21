#!/bin/bash
echo "== release =="
cat /etc/redos-release
echo "== tools =="
for t in dnf yum rpm rpmbuild curl gcc g++ python3 make git tar; do
  printf "%s: %s\n" "$t" "$(command -v $t || echo MISSING)"
done
echo "== cpu =="
nproc
echo "== rust =="
command -v cargo && cargo --version && rustc --version
echo "== osver =="
cat /etc/os-release
echo "== repos =="
ls /etc/yum.repos.d/
