#clean all
docker-compose -p app -f docker-compose-example-applications.yaml down --remove-orphans
docker-compose -p app -f docker-compose-example-sidecars.yaml down --remove-orphans
docker network rm docker_swir-net-aws
./aws-scripts/aws-kinesis-delete-stream.sh aws_processor_orders_blue 
./aws-scripts/aws-kinesis-delete-stream.sh aws_processor_inventory_green 
./aws-scripts/aws-kinesis-delete-stream.sh aws_processor_billing_blue 
./aws-scripts/aws-kinesis-delete-stream.sh aws_sink_green
./aws-scripts/aws-delete-table.sh "swir-locks"
