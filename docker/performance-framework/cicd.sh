
# Compile, build and generate necessary docker images
cd ../../
./cicd.sh
cd docker/performance-framework

cd swir-java-client
./gradlew clean bootJar
docker build --tag swir-java-client:v3 .

cd ../swir-kafka-sink
./gradlew clean bootJar
docker build --tag swir-kafka-sink:v3 .

cd ../swir-nats-sink
./gradlew clean bootJar
docker build --tag swir-nats-sink:v3 .

cd ../swir-grpc-client
./gradlew clean build installDist assembleDist
docker build --tag swir-grpc-client:v3 .

cd ../swir-grpc-sink
./gradlew clean build installDist assembleDist
docker build --tag swir-grpc-sink:v3 .

cd ..

docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans
# this should deploy the infrastructure
# Docker instance names/network name created by docker compose could change
docker-compose -f docker-compose-infr.yml -p docker up -d
 
# Create necessary topics for Kafka
sleep 5

docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request2 --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response2 --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic RequestNoSidecar --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic ResponseNoSidecar --partitions 2 --replication-factor 1

# this should deploy swir and other components
docker-compose  -f docker-compose-swir.yml -p pf up -d

#use these to produce and receive messasges

#Kafka test over REST
#docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100, "threads":10, "sidecarUrl":"http://pf_swir_1:8080","producerTopics":["ProduceToAppA","ProduceToAppC"],"subscriberTopics":["SubscribeToAppA","SubscribeToAppC"],"missedPackets":50}' -H "Content-Type: application/json" -X POST http://pf_swir-java-client_1:8090/test

#Nats test over REST
#docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100, "threads":10, "sidecarUrl":"http://pf_swir_1:8080","producerTopics":["ProduceToAppB","ProduceToAppD"],"subscriberTopics":["SubscribeToAppB","SubscribeToAppD"],"missedPackets":50}' -H "Content-Type: application/json" -X POST http://pf_swir-java-client_1:8090/test


#Kafka test over gRPC
#docker run -ti --network docker_swir-net  --rm -e sidecar_hostname=swir -e sidecar_port=50051 -e messages=100000 -e threads=10 -e client_request_topic=ProduceToAppA -e client_response_topic=SubscribeToAppA -e publish_type=[unary|bidi] swir-grpc-client:v3

#Nats test over gRPC
#docker run -ti --network docker_swir-net  --rm -e sidecar_hostname=swir -e sidecar_port=50051 -e messages=100000 -e threads=10 -e client_request_topic=ProduceToAppB -e client_response_topic=SubscribeToAppB -e publish_type=[unary|bidi] swir-grpc-client:v3

#gRPC to gRPC
#docker run -ti --network docker_swir-net  --rm -e sidecar_hostname=swir-grpc-sink -e sidecar_port=50052 -e messages=1000000 -e threads=200 -e client_request_topic=ProduceToAppA -e client_response_topic=SubscribeToAppA  swir-grpc-client:v3

#Kafka to Kafka
#docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":1000000, "threads":200, "sidecarUrl":"http://pf_swir_1:8080","clientTopic":"ProduceToAppA","testType":"kafka","missedPackets":50}' -H "Content-Type: application/json" -X POST http://pf_swir-java-client_1:8090/test

#Clean up
#docker-compose -p pf -f docker-compose-swir.yml down --remove-orphans


#Copy logs to file
#docker logs pf_swir_1 > logs 2>&1

#docker cp pf_swir_1:/pcap.logs ~/Workspace/rustycar/


#curl -v -d '{"messages":100, "threads":10, "sidecarUrl":"http://127.0.0.1:8080","producerTopics":["ProduceToKinesis"],"subscriberTopics":["SubscribeToKinesis"],"missedPackets":50}' -H "Content-Type: application/json" -X POST http://127.0.0.1:8090/test

