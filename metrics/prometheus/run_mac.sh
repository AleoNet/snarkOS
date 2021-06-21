cd mac || exit
docker build -t prometheus .
docker run -p 9090:9090 prometheus
