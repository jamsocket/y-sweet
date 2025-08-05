FROM localstack/localstack
COPY --chown=localstack ./script/init.sh /etc/localstack/init/ready.d/init-aws.sh
RUN chmod u+x /etc/localstack/init/ready.d/init-aws.sh