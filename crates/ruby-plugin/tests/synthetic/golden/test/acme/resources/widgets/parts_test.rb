# frozen_string_literal: true

require_relative "../../test_helper"

class Acme::Test::Resources::Widgets::PartsTest < Acme::Test::ResourceTest
  def test_retrieve_required_params
    response = @acme.widgets.parts.retrieve(0, widget_id: "widget_id")

    assert_pattern do
      response => Acme::Models::Widgets::PartRetrieveResponse
    end

    assert_pattern do
      response => {
        id: String,
        sku: String,
        error: Acme::ErrorInfo | nil
      }
    end
  end
end
