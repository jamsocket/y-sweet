#!/bin/sh

echo "S3 setup start!"
echo "Creating S3 bucket..."

aws --endpoint-url=http://aws:4566 s3 mb s3://local-asset-bucket/
echo 'bucket created!'
echo "S3 setup Done!"