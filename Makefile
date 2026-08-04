build:
	docker build -t scraping-service-api .

deploy:
	serverless deploy
