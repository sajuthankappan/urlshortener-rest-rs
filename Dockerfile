FROM ubuntu
ENV URLSHORTENER_MONGO_HOST="mongo"
COPY target/release/urlshortener_rest /bin/urlshortener_rest
EXPOSE 3000
CMD urlshortener_rest
