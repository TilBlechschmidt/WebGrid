from selenium import webdriver
from selenium.webdriver.support.ui import WebDriverWait as wait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.common.by import By
from time import sleep
import sys

if len(sys.argv) < 2:
    sys.exit("No port provided (first cli argument)!")

port = sys.argv[1]
url = "http://127.0.0.1:" + port
print("Connecting to: " + url)
browser = webdriver.Remote(url)

print("Session id: " + browser.session_id)

browser.get('https://duckduckgo.com')
search_form = browser.find_element_by_id('search_form_input_homepage')
search_form.send_keys('webgrid.dev')
search_form.submit()

wait(browser, 10).until(EC.presence_of_element_located((By.CLASS_NAME, 'result__a')))

results = browser.find_elements_by_class_name('result__a')

found = False
for result in results:
    found = result.text.find("WebGrid") > -1
    if found:
        break

browser.quit()

if not found:
    sys.exit("Did not find WebGrid in the search results :(")
else:
    print("Test successful!")
