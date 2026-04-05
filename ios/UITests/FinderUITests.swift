import XCTest

class FinderUITests: XCTestCase {
    let app = XCUIApplication()

    override func setUp() {
        continueAfterFailure = false
        app.launch()
    }

    func testCameraAndSearch() {
        // Wait for the camera view to appear
        let finderView = app.otherElements["finder_view"]
        let exists = finderView.waitForExistence(timeout: 10)

        sleep(2) // Let camera initialize

        // Type a search query
        let textField = app.textFields.firstMatch
        if textField.waitForExistence(timeout: 5) {
            textField.tap()
            textField.typeText("my keys")
            sleep(1)
        }

        // Tap search button
        let searchButton = app.buttons.firstMatch
        if searchButton.exists {
            searchButton.tap()
            sleep(3) // Let detection run
        }

        // Take screenshot for demo
        let screenshot = app.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
