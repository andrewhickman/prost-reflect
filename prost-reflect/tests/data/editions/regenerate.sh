#!/bin/sh
# Rebuilds the descriptors and the expected bytes from the .proto files with protoc, which is
# what makes these tests an independent check of the encoding each feature selects.
set -eu
cd "$(dirname "$0")"
for f in features inherited legacy_json edition_2024; do
  protoc --descriptor_set_out="$f.binpb" -I. "$f.proto"
done
protoc --encode=editions.Fields -I. features.proto <values.txtpb >values.binpb
protoc --encode=editions.inherited.FileLevel -I. inherited.proto <inherits_delimited.txtpb >inherits_delimited.binpb
