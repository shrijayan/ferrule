# frozen_string_literal: true

require_relative "../test_helper"

class Acme::Test::Resources::WidgetsTest < Acme::Test::ResourceTest
  def test_create_required_params
    response = @acme.widgets.create(name: "name")

    assert_pattern do
      response => Acme::Widget
    end

    assert_pattern do
      response => {
        id: String,
        created_at: Time,
        name: String,
        status: Acme::Widget::Status,
        homepage_url: String | nil,
        labels: ^(Acme::Internal::Type::HashOf[Acme::LabelsItem]) | nil,
        parts: ^(Acme::Internal::Type::ArrayOf[Acme::Widget::Part]) | nil,
        sku: String | nil,
        weight_kg: Float | nil
      }
    end
  end

  def test_retrieve
    response = @acme.widgets.retrieve("widget_id")

    assert_pattern do
      response => Acme::Widget
    end

    assert_pattern do
      response => {
        id: String,
        created_at: Time,
        name: String,
        status: Acme::Widget::Status,
        homepage_url: String | nil,
        labels: ^(Acme::Internal::Type::HashOf[Acme::LabelsItem]) | nil,
        parts: ^(Acme::Internal::Type::ArrayOf[Acme::Widget::Part]) | nil,
        sku: String | nil,
        weight_kg: Float | nil
      }
    end
  end

  def test_list
    response = @acme.widgets.list

    assert_pattern do
      response => Acme::Models::WidgetListResponse
    end

    assert_pattern do
      response => {
        data: ^(Acme::Internal::Type::ArrayOf[Acme::Widget]),
        next_cursor: String | nil
      }
    end
  end

  def test_delete
    response = @acme.widgets.delete("widget_id")

    assert_pattern do
      response => nil
    end
  end
end
