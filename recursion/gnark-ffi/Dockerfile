FROM golang:latest
WORKDIR /OUT_DIR
COPY ./go .
# Run the build command
RUN CGO_ENABLED=1 go build -o libsp1gnark.a -buildmode=c-archive .