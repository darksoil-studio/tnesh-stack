#!/usr/bin/bash
set -e

DIR=$(pwd)


nix shell --accept-flake-config .#scaffold-tnesh-zome --command bash -c "
cd /tmp
rm -rf posts-tnesh
mkdir posts-tnesh
cd posts-tnesh
scaffold-tnesh-zome --zome-name posts --github-organization darksoil-studio --cachix-cache darksoil-studio --npm-organization darksoil-studio 
"

cd /tmp/posts-tnesh/posts-zome

nix develop --no-update-lock-file --accept-flake-config --override-input tnesh-stack "path:$DIR" --command bash -c "
set -e

hc-scaffold entry-type post --zome posts_integrity --reference-entry-hash false --crud crud --link-from-original-to-each-update true --fields title:String:TextField,needs:Vec\<String\>:TextField
hc-scaffold entry-type comment --zome posts_integrity  --reference-entry-hash false --crud crud --link-from-original-to-each-update false --fields post_hash:ActionHash::Post
hc-scaffold entry-type like --zome posts_integrity  --reference-entry-hash false --crud crd --fields like_hash:Option\<ActionHash\>::Like,agent:AgentPubKey:SearchAgent
hc-scaffold entry-type certificate --zome posts_integrity  --reference-entry-hash false --crud cr --fields post_hash:ActionHash::Post,agent:AgentPubKey::certified,certifications_hashes:Vec\<EntryHash\>::Certificate,certificate_type:Enum::CertificateType:TypeOne.TypeTwo,dna_hash:DnaHash

hc-scaffold collection --zome posts_integrity global all_posts post 
hc-scaffold collection --zome posts_integrity by-author posts_by_author post
hc-scaffold collection --zome posts_integrity global all_posts_entry_hash post:EntryHash
hc-scaffold collection --zome posts_integrity by-author posts_by_author_entry_hash post:EntryHash

hc-scaffold link-type --zome posts_integrity post like --delete true --bidirectional false
hc-scaffold link-type --zome posts_integrity comment like --delete true --bidirectional false
hc-scaffold link-type --zome posts_integrity certificate like --delete false --bidirectional false
hc-scaffold link-type --zome posts_integrity agent:creator post --delete false --bidirectional false

git add .

nix flake lock

pnpm i

nix run github:darksoil-studio/file-storage/main-0.4#scaffold -- --ci

pnpm -F @darksoil-studio/posts format
pnpm -F @darksoil-studio/posts lint
pnpm -F @darksoil-studio/posts build

pnpm i

pnpm build:happ
pnpm -F tests test
"
