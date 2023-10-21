#!/bin/sh

set -e

docker rm -f minio
docker rm -f minio-mc

docker run -d --network host --name minio \
  quay.io/minio/minio server /data --console-address ":9001"

docker run -d --network host --name minio-mc --entrypoint=/bin/sh minio/mc -c 'while true; do sleep 1000; done'
docker exec minio-mc mc alias set mycloud http://localhost:9000 minioadmin minioadmin
docker exec minio-mc mc mb mycloud/ysweet-testing-y-sweet-data


export Y_SWEET_S3_BUCKET_PREFIX=testing
export Y_SWEET_S3_BUCKET_NAME=ysweet-testing-y-sweet-data
export Y_SWEET_MINIO_PORT=9000

npm test

docker stop minio
docker rm minio
